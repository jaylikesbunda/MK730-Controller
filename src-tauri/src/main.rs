// Hide the console window in release builds on Windows.
#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

//! MK730 Controller Tauri backend (V2 HID protocol, OpenRGB-derived).

use mk730_core::{
    default_mk730_tkl, transport::demo_device, CANDIDATE_PIDS, SUPPORTED_VIDS, CoreError,
    KeyDef, Macro, Profile,
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
    colors: Mutex<[[u8; 3]; 255]>,
    initialized: Mutex<bool>,
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

fn friendly_name(pid: u16) -> &'static str {
    match pid {
        0x008F => "MK730",
        0x0067 => "MK750",
        0x0069 => "MK850",
        0x009F | 0x0147 => "CK530",
        0x0145 | 0x007F => "CK550 / CK552",
        0x0089 => "SK630",
        0x008D => "SK650",
        0x0149 | 0x014B => "SK622",
        0x0157 | 0x0159 => "SK620",
        0x015D => "SK652",
        0x01AB => "SK653",
        _ => "Cooler Master keyboard",
    }
}

#[tauri::command]
fn list_devices(state: State<AppState>) -> Vec<mk730_core::transport::DeviceInfo> {
    use std::collections::HashMap;
    let mut found = Vec::new();
    let mut hid_total = 0usize;
    let mut cm_any: Vec<String> = Vec::new();
    // hidapi lists each HID collection separately, so one keyboard appears
    // 4-6 times (typing, media keys, vendor control...). Group by physical
    // device (vid+pid+serial) and show one row.
    if let Ok(api) = hidapi::HidApi::new() {
        let mut seen: HashMap<String, mk730_core::transport::DeviceInfo> = HashMap::new();
        for dev in api.device_list() {
            hid_total += 1;
            if SUPPORTED_VIDS.contains(&dev.vendor_id()) {
                let pid = dev.product_id();
                let iface_raw = dev.interface_number();
                let iface = if iface_raw < 0 { 255u8 } else { iface_raw as u8 };
                cm_any.push(format!("{:04x} if{}", pid, iface));
                let serial = dev.serial_number().unwrap_or("").to_string();
                let key = format!("{:04x}:{:04x}:{}", dev.vendor_id(), pid, serial);
                seen.entry(key).or_insert_with(|| {
                    // Firmware sometimes reports the serial as product string on
                    // one collection — always prefer the friendly PID name.
                    mk730_core::transport::DeviceInfo {
                        vid: dev.vendor_id(),
                        pid,
                        interface: 1,
                        path: format!("{:?}", dev.path()),
                        product: friendly_name(pid).to_string(),
                        demo: false,
                    }
                });
            }
        }
        found.extend(seen.into_values());
    }
    // rusb fallback probe: any device with CM VID, any PID.
    let mut rusb_any: Vec<String> = Vec::new();
    if found.is_empty() {
        if let Ok(ctx) = rusb::Context::new() {
            if let Ok(list) = ctx.devices() {
                for h in list.iter() {
                    if let Ok(desc) = h.device_descriptor() {
                        if SUPPORTED_VIDS.contains(&desc.vendor_id()) {
                            rusb_any.push(format!("{:04x}", desc.product_id()));
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
        push_log(
            &state,
            format!(
                "list_devices: none with VID 2516 (hid_total={} rusb_cm={:?}) — demo",
                hid_total, rusb_any
            ),
        );
        return vec![demo_device()];
    }
    {
        let mut d = state.dev.lock();
        d.connected = true;
        d.demo = false;
    }
    push_log(
        &state,
        format!(
            "list_devices: {} CM device(s) [{}] hid_total={}",
            found.len(),
            cm_any.join(","),
            hid_total
        ),
    );
    found
}

#[derive(Debug, Clone, Serialize)]
struct UsbDebug {
    hid_total: usize,
    hid_cm: Vec<serde_json::Value>,
    rusb_cm: Vec<serde_json::Value>,
    rusb_error: String,
    hint: String,
}

#[tauri::command]
fn debug_usb(state: State<AppState>) -> UsbDebug {
    let mut hid_total = 0usize;
    let mut hid_cm = Vec::new();
    if let Ok(api) = hidapi::HidApi::new() {
        for dev in api.device_list() {
            hid_total += 1;
            if SUPPORTED_VIDS.contains(&dev.vendor_id()) {
                hid_cm.push(serde_json::json!({
                    "vid": format!("{:04x}", dev.vendor_id()),
                    "pid": format!("{:04x}", dev.product_id()),
                    "interface": dev.interface_number(),
                    "usage_page": dev.usage_page(),
                    "usage": dev.usage(),
                    "product": dev.product_string().unwrap_or(""),
                    "manufacturer": dev.manufacturer_string().unwrap_or(""),
                    "path": format!("{:?}", dev.path()),
                }));
            }
        }
    }
    let mut rusb_cm = Vec::new();
    let mut rusb_error = String::new();
    match rusb::Context::new() {
        Ok(ctx) => match ctx.devices() {
            Ok(list) => {
                for h in list.iter() {
                    if let Ok(desc) = h.device_descriptor() {
                        if SUPPORTED_VIDS.contains(&desc.vendor_id()) {
                            rusb_cm.push(serde_json::json!({
                                "vid": format!("{:04x}", desc.vendor_id()),
                                "pid": format!("{:04x}", desc.product_id()),
                                "bus": h.bus_number(),
                                "addr": h.address(),
                            }));
                        }
                    }
                }
            }
            Err(e) => rusb_error = format!("devices: {}", e),
        },
        Err(e) => rusb_error = format!("ctx: {}", e),
    }
    push_log(
        &state,
        format!(
            "debug_usb: hid_total={} cm_hid={} cm_rusb={}",
            hid_total,
            hid_cm.len(),
            rusb_cm.len()
        ),
    );
    UsbDebug {
        hid_total,
        hid_cm,
        rusb_cm,
        rusb_error,
        hint: "If empty: check Device Manager -> Keyboards -> Details -> Hardware Ids for VID_2516&PID_XXXX, or PowerShell: Get-CimInstance Win32_PnPEntity | Where-Object {$_.DeviceID -like '*VID_2516*'} | Select-Object Name,DeviceID".to_string(),
    }
}

fn control_path() -> Option<String> {
    let api = hidapi::HidApi::new().ok()?;
    let mut fallback: Option<String> = None;
    let mut best: Option<String> = None;
    for dev in api.device_list() {
        if !SUPPORTED_VIDS.contains(&dev.vendor_id()) {
            continue;
        }
        let pid = dev.product_id();
        let up = dev.usage_page();
        let path = dev.path().to_bytes_with_nul();
        let path = String::from_utf8_lossy(path).into_owned();
        // Prefer MK730 on the vendor control collection.
        if pid == 0x008F && up == 0xFF00 {
            return Some(path);
        }
        if CANDIDATE_PIDS.contains(&pid) && up == 0xFF00 && best.is_none() {
            best = Some(path.clone());
        }
        if fallback.is_none() {
            fallback = Some(path);
        }
    }
    best.or(fallback)
}

fn hid_send_many(state: &State<AppState>, payloads: &[Vec<u8>], label: &str) -> Result<String, CoreError> {
    let short = payloads
        .first()
        .map(|p| mk730_core::proto::hexv(p))
        .unwrap_or_default();
    if state.dev.lock().demo {
        push_log(state, format!("{} [preview]: {}", label, short));
        return Ok(short);
    }
    // Open once per batch — opening per packet was the main paint lag.
    let path = control_path().ok_or_else(|| {
        CoreError::from("keyboard control interface not found — check cable and close other lighting apps")
    })?;
    let api = hidapi::HidApi::new().map_err(|e| CoreError::from(format!("hid init: {}", e)))?;
    let cpath = std::ffi::CString::new(path.trim_matches('"').trim_matches('\0'))
        .map_err(|e| CoreError::from(format!("bad device path: {}", e)))?;
    let dev = api
        .open_path(&cpath)
        .map_err(|e| CoreError::from(format!("open keyboard: {} (close Portal / other apps)", e)))?;
    let mut last = short.clone();
    for p in payloads {
        let mut report = [0u8; 65];
        let n = p.len().min(64);
        report[1..1 + n].copy_from_slice(&p[..n]);
        dev.write(&report)
            .map_err(|e| CoreError::from(format!("write failed: {}", e)))?;
        let mut resp = [0u8; 65];
        let _ = dev.read_timeout(&mut resp, 25);
        last = mk730_core::proto::hexv(p);
    }
    push_log(state, format!("{} ({} packets)", label, payloads.len()));
    Ok(last)
}

fn hid_send(state: &State<AppState>, payload: &[u8], label: &str) -> Result<String, CoreError> {
    hid_send_many(state, &[payload.to_vec()], label)
}

fn v2_ensure_init(state: &State<AppState>) {
    if state.initialized.lock().clone() || state.dev.lock().demo {
        return;
    }
    let mut init = mk730_core::proto::v2::init_mk730();
    init.push(mk730_core::proto::v2::set_led_control(true));
    if hid_send_many(state, &init, "init").is_ok() {
        *state.initialized.lock() = true;
    }
}

fn v2_push_map(state: &State<AppState>) -> Result<String, CoreError> {
    v2_ensure_init(state);
    let map = *state.colors.lock();
    let pkts = mk730_core::proto::v2::direct_packets(&map);
    hid_send_many(state, &pkts, "paint")
}

#[tauri::command]
fn set_mode(state: State<AppState>, mode: u8) -> Result<String, CoreError> {
    let payload: Vec<u8> = match mode {
        0x00 => vec![0x41, 0x00],
        0x01 => vec![0x41, 0x80],
        0x02 => vec![0x41, 0x05],
        0x03 => vec![0x41, 0x05],
        _ => return Err(CoreError::from("unknown mode")),
    };
    let h = hid_send(&state, &payload, "mode")?;
    state.dev.lock().mode = mode;
    Ok(h)
}

#[tauri::command]
fn set_full_color(state: State<AppState>, r: u8, g: u8, b: u8) -> Result<String, CoreError> {
    *state.colors.lock() = [[r, g, b]; 255];
    v2_push_map(&state)
}

#[tauri::command]
fn set_key_color(state: State<AppState>, id: u16, r: u8, g: u8, b: u8) -> Result<String, CoreError> {
    // Single-key paint updates the cached map, then pushes the FULL map —
    // V2 firmware has no single-LED command, which is why per-key felt broken.
    {
        let mut map = state.colors.lock();
        if (id as usize) < 255 {
            map[id as usize] = [r, g, b];
        }
    }
    v2_push_map(&state)
}

#[tauri::command]
fn set_colormap(state: State<AppState>, colors: Vec<[u8; 3]>) -> Result<String, CoreError> {
    // colors[i] = LED value i (255 entries). Frontend builds this from the
    // keymap so clicks land on the right physical key.
    {
        let mut map = state.colors.lock();
        for (i, c) in colors.iter().take(255).enumerate() {
            map[i] = *c;
        }
    }
    v2_push_map(&state)
}

#[derive(Debug, Clone, Deserialize)]
struct EffectParamsIn {
    eid: u8,
    level: u8,
    brightness: u8,
    dir: u8,
    c1: [u8; 3],
    c2: [u8; 3],
}

#[tauri::command]
fn set_effect(state: State<AppState>, eid: u8) -> Result<String, CoreError> {
    v2_ensure_init(&state);
    let seq = mk730_core::proto::v2::effect_sequence(eid, 3, 255, 0x00, [124, 58, 237], [0, 0, 0]);
    let mut last = String::new();
    for p in seq.iter() {
        last = hid_send(&state, p, "effect")?;
    }
    Ok(last)
}

#[tauri::command]
fn set_effect_params(state: State<AppState>, p: EffectParamsIn) -> Result<String, CoreError> {
    v2_ensure_init(&state);
    let level = p.level.clamp(1, 5);
    let seq = mk730_core::proto::v2::effect_sequence(p.eid, level, p.brightness, p.dir, p.c1, p.c2);
    let mut last = String::new();
    for pkt in seq.iter() {
        last = hid_send(&state, pkt, "effect")?;
    }
    Ok(last)
}

#[tauri::command]
fn set_active_profile(state: State<AppState>, id: u8) -> Result<String, CoreError> {
    if id > 4 {
        return Err(CoreError::from("profile id 0..4"));
    }
    let h = hid_send(&state, &[0x51, 0x00, 0x00, 0x00, id], "profile")?;
    state.dev.lock().active_profile = id;
    Ok(h)
}

#[tauri::command]
fn save_profile_fw(state: State<AppState>) -> Result<String, CoreError> {
    hid_send(&state, &[0x50, 0x55], "save")
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
    // V2: select profile, push effect sequence, save.
    v2_ensure_init(&state);
    hid_send(&state, &[0x51, 0x00, 0x00, 0x00, p.id], "profile")?;
    let level = 3u8;
    let seq = mk730_core::proto::v2::effect_sequence(
        p.effect_id, level, 255, 0x00,
        [p.params.color1.r, p.params.color1.g, p.params.color1.b],
        [p.params.color2.r, p.params.color2.g, p.params.color2.b],
    );
    let mut h = String::new();
    for pkt in seq.iter() {
        h = hid_send(&state, pkt, "profile effect")?;
    }
    hid_send(&state, &[0x50, 0x55], "save")?;
    state.profiles.lock().insert(p.id, p);
    Ok(h)
}

#[tauri::command]
fn win_minimize(window: tauri::Window) {
    let _ = window.minimize();
}

#[tauri::command]
fn win_toggle_maximize(window: tauri::Window) {
    if window.is_maximized().unwrap_or(false) {
        let _ = window.unmaximize();
    } else {
        let _ = window.maximize();
    }
}

#[tauri::command]
fn win_close(window: tauri::Window) {
    window.close().ok();
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
            colors: Mutex::new([[0u8; 3]; 255]),
            initialized: Mutex::new(false),
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            list_devices,
            debug_usb,
            win_minimize,
            win_toggle_maximize,
            win_close,
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
