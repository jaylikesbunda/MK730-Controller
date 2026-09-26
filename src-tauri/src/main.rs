//! MK730 Controller Tauri backend.
//! Transport: try hidapi enumeration for CM VID, else demo mode so the UI
//! is always usable. Actual 64B interrupt writes go via rusb Interface 1
//! (OUT 0x04) on Linux; on Windows via hidapi feature reports path + rusb
//! when WinUSB driver is bound. All commands log hex for capture comparison.

use mk730_core::{
    default_mk730_tkl, transport::demo_device, CANDIDATE_PIDS, CM_VID, CoreError, KeyDef, Macro,
    Profile,
};
use parking_lot::Mutex;
use rusb::UsbContext;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DeviceState {
    connected: bool,
    demo: bool,
    firmware: String,
    mode: u8,
    active_profile: u8,
    log: Vec<String>,
}

impl Default for DeviceState {
    fn default() -> Self {
        Self {
            connected: false,
            demo: true,
            firmware: "demo-0.1.0".to_string(),
            mode: 0x01,
            active_profile: 0,
            log: vec!["boot: demo mode (no hardware claimed)".to_string()],
        }
    }
}

struct AppState {
    dev: Mutex<DeviceState>,
    profiles: Mutex<HashMap<u8, Profile>>,
    macros: Mutex<HashMap<String, Macro>>,
}

fn push_log(state: &State<AppState>, msg: String) {
    let mut d = state.dev.lock();
    d.log.push(msg);
    if d.log.len() > 300 {
        let excess = d.log.len() - 300;
        d.log.drain(0..excess);
    }
}

#[derive(Debug, Clone, Serialize)]
struct Status {
    connected: bool,
    demo: bool,
    firmware: String,
    mode: u8,
    active_profile: u8,
    log_tail: Vec<String>,
}

#[tauri::command]
fn get_status(state: State<AppState>) -> Status {
    let d = state.dev.lock();
    Status {
        connected: d.connected,
        demo: d.demo,
        firmware: d.firmware.clone(),
        mode: d.mode,
        active_profile: d.active_profile,
        log_tail: d.log.iter().rev().take(80).cloned().collect(),
    }
}

#[tauri::command]
fn list_devices(state: State<AppState>) -> Vec<mk730_core::transport::DeviceInfo> {
    let mut found = Vec::new();
    // hidapi enumeration (works on Linux + Windows without claiming)
    if let Ok(api) = hidapi::HidApi::new() {
        for dev in api.device_list() {
            if dev.vendor_id() == CM_VID
                && CANDIDATE_PIDS.contains(&dev.product_id())
            {
                found.push(mk730_core::transport::DeviceInfo {
                    vid: dev.vendor_id(),
                    pid: dev.product_id(),
                    interface: dev.interface_number() as u8,
                    path: format!("{:?}", dev.path()),
                    product: dev.product_string().unwrap_or("Cooler Master").to_string(),
                    demo: false,
                });
            }
        }
    }
    // rusb fallback probe (interface 1 present?)
    if found.is_empty() {
        if let Ok(ctx) = rusb::Context::new() {
            if let Ok(list) = ctx.devices() {
                for h in list.iter() {
                    if let Ok(desc) = h.device_descriptor() {
                        if desc.vendor_id() == CM_VID
                            && CANDIDATE_PIDS.contains(&desc.product_id())
                        {
                            found.push(mk730_core::transport::DeviceInfo {
                                vid: desc.vendor_id(),
                                pid: desc.product_id(),
                                interface: 1,
                                path: format!(
                                    "bus{:03}-dev{:03}",
                                    h.bus_number(),
                                    h.address()
                                ),
                                product: "Cooler Master keyboard".to_string(),
                                demo: false,
                            });
                        }
                    }
                }
            }
        }
    }
    if found.is_empty() {
        push_log(&state, "list_devices: none found, demo device".to_string());
        return vec![demo_device()];
    }
    {
        let mut d = state.dev.lock();
        d.connected = true;
        d.demo = false;
    }
    push_log(&state, format!("list_devices: {} candidate(s)", found.len()));
    found
}

fn send_packet(state: &State<AppState>, pkt: &[u8; 64], label: &str) -> Result<String, CoreError> {
    let hex = mk730_core::proto::hex(pkt);
    let demo = state.dev.lock().demo;
    if demo {
        push_log(state, format!("{} [demo]: {}", label, &hex[..48.min(hex.len())]));
        return Ok(hex);
    }
    // Real path: claim Interface 1 via rusb and interrupt-write to EP 0x04.
    // Kept defensive: any failure returns a clear error, never panics.
    match try_rusb_write(pkt) {
        Ok(_) => {
            push_log(state, format!("{} [usb]: {}", label, &hex[..48.min(hex.len())]));
            Ok(hex)
        }
        Err(e) => {
            push_log(state, format!("{} FAILED: {}", label, e));
            Err(CoreError::from(e))
        }
    }
}

fn try_rusb_write(pkt: &[u8; 64]) -> Result<(), String> {
    let ctx = rusb::Context::new().map_err(|e| format!("rusb ctx: {}", e))?;
    let list = ctx.devices().map_err(|e| format!("rusb list: {}", e))?;
    for handle in list.iter() {
        let desc = handle.device_descriptor().map_err(|e| format!("desc: {}", e))?;
        if desc.vendor_id() != CM_VID || !CANDIDATE_PIDS.contains(&desc.product_id()) {
            continue;
        }
        let h = handle.open().map_err(|e| format!("open: {}", e))?;
        // Detach kernel driver on Interface 1 only (Linux). Never touch IF 0 (typing).
        #[cfg(target_os = "linux")]
        {
            if h.kernel_driver_active(1).unwrap_or(false) {
                let _ = h.detach_kernel_driver(1);
            }
        }
        h.claim_interface(1).map_err(|e| format!("claim IF1 (need udev/WinUSB, see docs): {}", e))?;
        // EP 0x04 OUT, 1000ms timeout
        let n = h.write_interrupt(0x04, pkt, std::time::Duration::from_millis(1000))
            .map_err(|e| format!("write_interrupt 0x04: {}", e))?;
        h.release_interface(1).ok();
        if n != 64 {
            return Err(format!("short write {}", n));
        }
        return Ok(());
    }
    Err("no MK730 candidate matched VID/PID list — check lsusb and CANDIDATE_PIDS".to_string())
}

#[tauri::command]
fn set_mode(state: State<AppState>, mode: u8) -> Result<String, CoreError> {
    let m = match mode {
        0x00 => mk730_core::proto::ControlMode::Firmware,
        0x01 => mk730_core::proto::ControlMode::Effect,
        0x02 => mk730_core::proto::ControlMode::Manual,
        0x03 => mk730_core::proto::ControlMode::Profile,
        _ => return Err(CoreError::from("mode must be 0..3")),
    };
    let pkt = mk730_core::proto::build_set_mode(m);
    let h = send_packet(&state, &pkt, &format!("41 {:02x}", mode))?;
    state.dev.lock().mode = mode;
    Ok(h)
}

#[tauri::command]
fn set_full_color(state: State<AppState>, r: u8, g: u8, b: u8) -> Result<String, CoreError> {
    // ensure manual mode first (best-effort, ignore error in demo)
    let _ = set_mode(state.clone(), 0x02);
    let pkt = mk730_core::proto::build_manual_full_color(r, g, b);
    send_packet(&state, &pkt, "c0 00 full")
}

#[tauri::command]
fn set_key_color(state: State<AppState>, id: u8, r: u8, g: u8, b: u8) -> Result<String, CoreError> {
    let _ = set_mode(state.clone(), 0x02);
    let pkt = mk730_core::proto::build_manual_key(id, r, g, b);
    send_packet(&state, &pkt, "c0 01 key")
}

#[tauri::command]
fn set_colormap(state: State<AppState>, colors: Vec<[u8; 3]>) -> Result<String, CoreError> {
    let _ = set_mode(state.clone(), 0x02);
    let pkts = mk730_core::proto::build_colormap_packets(&colors, true);
    let mut last = String::new();
    for (i, p) in pkts.iter().enumerate() {
        last = send_packet(&state, p, &format!("c0 02 [{}/8]", i + 1))?;
    }
    Ok(last)
}

#[derive(Debug, Clone, Deserialize)]
struct EffectParamsIn {
    eid: u8,
    p1: u8,
    p2: u8,
    p3: u8,
    c1: [u8; 3],
    c2: [u8; 3],
}

#[tauri::command]
fn set_effect(state: State<AppState>, eid: u8) -> Result<String, CoreError> {
    let _ = set_mode(state.clone(), 0x01);
    let pkt = mk730_core::proto::build_set_effect(eid);
    send_packet(&state, &pkt, "51 28 effect")
}

#[tauri::command]
fn set_effect_params(state: State<AppState>, p: EffectParamsIn) -> Result<String, CoreError> {
    let _ = set_mode(state.clone(), 0x01);
    let params = mk730_core::proto::EffectParams {
        p1_speed: p.p1,
        p2: p.p2,
        p3: p.p3,
        color1: mk730_core::proto::Rgb::new(p.c1[0], p.c1[1], p.c1[2]),
        color2: mk730_core::proto::Rgb::new(p.c2[0], p.c2[1], p.c2[2]),
        multilayer: 0x00,
    };
    let pkt = mk730_core::proto::build_set_effect_params(p.eid, &params);
    let h = send_packet(&state, &pkt, "51 2c params")?;
    Ok(h)
}

#[tauri::command]
fn set_active_profile(state: State<AppState>, id: u8) -> Result<String, CoreError> {
    if id > 4 {
        return Err(CoreError::from("profile id 0..4"));
    }
    let _ = set_mode(state.clone(), 0x03);
    let pkt = mk730_core::proto::build_set_active_profile(id);
    let h = send_packet(&state, &pkt, "51 00 profile")?;
    state.dev.lock().active_profile = id;
    let _ = set_mode(state.clone(), 0x01);
    Ok(h)
}

#[tauri::command]
fn save_profile_fw(state: State<AppState>) -> Result<String, CoreError> {
    let pkt = mk730_core::proto::build_save_profile();
    send_packet(&state, &pkt, "50 55 save")
}

#[tauri::command]
fn get_keymap() -> Vec<KeyDef> {
    default_mk730_tkl()
}

#[tauri::command]
fn get_profiles(state: State<AppState>) -> Vec<Profile> {
    let map = state.profiles.lock();
    let mut v: Vec<Profile> = map.values().cloned().collect();
    v.sort_by_key(|p| p.id);
    v
}

#[tauri::command]
fn upsert_profile(state: State<AppState>, p: Profile) -> Result<Vec<Profile>, CoreError> {
    if p.id > 4 {
        return Err(CoreError::from("profile id 0..4"));
    }
    state.profiles.lock().insert(p.id, p);
    Ok(get_profiles(state))
}

#[tauri::command]
fn get_macros(state: State<AppState>) -> Vec<Macro> {
    let mut v: Vec<Macro> = state.macros.lock().values().cloned().collect();
    v.sort_by(|a, b| a.trigger.cmp(&b.trigger));
    v
}

#[tauri::command]
fn save_macro(state: State<AppState>, m: Macro) -> Result<Vec<Macro>, CoreError> {
    // Always succeeds locally; firmware sync explicitly pending RE.
    let mut map = state.macros.lock();
    let mut m2 = m.clone();
    if m2.fw_state.is_empty() {
        m2.fw_state = "local_only".to_string();
    }
    map.insert(m2.id.clone(), m2);
    push_log(
        &state,
        format!("macro saved locally: {} ({})", m.trigger, m.id),
    );
    drop(map);
    Ok(get_macros(state))
}

#[tauri::command]
fn delete_macro(state: State<AppState>, id: String) -> Vec<Macro> {
    state.macros.lock().remove(&id);
    push_log(&state, format!("macro deleted: {}", id));
    get_macros(state)
}

#[tauri::command]
fn apply_profile_to_device(
    state: State<AppState>,
    p: Profile,
) -> Result<String, CoreError> {
    // Sequence mirrors Portal: profile mode -> effect+params+colormap -> save -> effect mode
    let _ = set_mode(state.clone(), 0x03);
    let _ = send_packet(
        &state,
        &mk730_core::proto::build_set_active_profile(p.id),
        "apply profile id",
    )?;
    let _ = send_packet(
        &state,
        &mk730_core::proto::build_set_effect(p.effect_id),
        "apply effect",
    )?;
    let _ = send_packet(
        &state,
        &mk730_core::proto::build_set_effect_params(p.effect_id, &p.params),
        "apply params",
    )?;
    let col = mk730_core::proto::build_colormap_packets(&p.colormap, false);
    for (i, pkt) in col.iter().enumerate() {
        let _ = send_packet(&state, pkt, &format!("51 a8 [{}/8]", i + 1))?;
    }
    let h = save_profile_fw(state.clone())?;
    let _ = set_mode(state.clone(), 0x01);
    state.profiles.lock().insert(p.id, p);
    Ok(h)
}

pub fn run() {
    let mut pmap = HashMap::new();
    for p in Profile::default_set() {
        pmap.insert(p.id, p);
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            dev: Mutex::new(DeviceState::default()),
            profiles: Mutex::new(pmap),
            macros: Mutex::new(HashMap::new()),
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            list_devices,
            set_mode,
            set_full_color,
            set_key_color,
            set_colormap,
            set_effect,
            set_effect_params,
            set_active_profile,
            save_profile_fw,
            get_keymap,
            get_profiles,
            upsert_profile,
            apply_profile_to_device,
            get_macros,
            save_macro,
            delete_macro
        ])
        .run(tauri::generate_context!())
        .expect("tauri run failed");
}

fn main() {
    run();
}
