// Hide the console window in release builds on Windows.
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

//! MK730 Controller Tauri backend (V2 HID protocol, OpenRGB-derived).

use mk730_core::{
    default_mk730_tkl, transport::demo_device, CoreError, KeyDef, Macro, Profile, CANDIDATE_PIDS,
    SUPPORTED_VIDS,
};
use parking_lot::Mutex;
use rusb::UsbContext;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{Manager, State};

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
    macros: Arc<Mutex<HashMap<String, Macro>>>,
    colors: Mutex<[[u8; 3]; 255]>,
    hardware: Mutex<HardwareState>,
    transport: Mutex<()>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LightingMode {
    Unknown,
    Direct,
    Effect,
}

struct HardwareState {
    initialized: bool,
    mode: LightingMode,
}

fn log_path() -> std::path::PathBuf {
    std::env::temp_dir().join("mk730-controller.log")
}

/// Append to a plain-text log in %TEMP% so behaviour can be inspected without
/// the app UI (useful when the window looks frozen).
fn file_log(msg: &str) {
    use std::io::Write;
    let p = log_path();
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&p)
    {
        let _ = writeln!(f, "{} {}", chrono_stamp(), msg);
    }
}

fn chrono_stamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let d = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);
    format!("[{:.3}]", d)
}

fn push_log(state: &State<AppState>, msg: String) {
    file_log(&msg);
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

/// Scan for Cooler Master keyboards. Returns one entry per physical device
/// plus a summary string of every CM collection seen.
fn probe_devices() -> Vec<mk730_core::transport::DeviceInfo> {
    use std::collections::HashMap;
    let mut found = Vec::new();
    let mut seen: HashMap<String, mk730_core::transport::DeviceInfo> = HashMap::new();
    let mut cm_any: Vec<String> = Vec::new();
    let mut hid_total = 0usize;

    if let Ok(api) = hidapi::HidApi::new() {
        for dev in api.device_list() {
            hid_total += 1;
            if SUPPORTED_VIDS.contains(&dev.vendor_id()) {
                let pid = dev.product_id();
                let iface_raw = dev.interface_number();
                let iface = if iface_raw < 0 {
                    255u8
                } else {
                    iface_raw as u8
                };
                cm_any.push(format!(
                    "{:04x}/if{}/up{:04x}",
                    pid,
                    iface,
                    dev.usage_page()
                ));
                let serial = dev.serial_number().unwrap_or("").to_string();
                // Fall back to the device path when collections report no serial,
                // so every collection collapses into one row per keyboard.
                let key = if serial.is_empty() {
                    format!("{:04x}:{:04x}", dev.vendor_id(), pid)
                } else {
                    format!("{:04x}:{:04x}:{}", dev.vendor_id(), pid, serial)
                };
                seen.entry(key)
                    .or_insert_with(|| mk730_core::transport::DeviceInfo {
                        vid: dev.vendor_id(),
                        pid,
                        interface: 1,
                        path: String::from_utf8_lossy(dev.path().to_bytes_with_nul())
                            .trim_end_matches('\0')
                            .to_string(),
                        product: friendly_name(pid).to_string(),
                        demo: false,
                    });
            }
        }
        found.extend(seen.into_values());
    }

    // rusb fallback for cases where hidapi is unavailable/blocked.
    if found.is_empty() {
        if let Ok(ctx) = rusb::Context::new() {
            if let Ok(list) = ctx.devices() {
                for h in list.iter() {
                    if let Ok(desc) = h.device_descriptor() {
                        if SUPPORTED_VIDS.contains(&desc.vendor_id()) {
                            cm_any.push(format!("{:04x}/usb", desc.product_id()));
                            found.push(mk730_core::transport::DeviceInfo {
                                vid: desc.vendor_id(),
                                pid: desc.product_id(),
                                interface: 1,
                                path: format!("bus{:03}-dev{:03}", h.bus_number(), h.address()),
                                product: friendly_name(desc.product_id()).to_string(),
                                demo: false,
                            });
                        }
                    }
                }
            }
        }
    }

    file_log(&format!(
        "probe: hid_total={} cm=[{}] found={}",
        hid_total,
        cm_any.join(","),
        found.len()
    ));
    found
}

#[tauri::command]
fn list_devices(state: State<AppState>) -> Vec<mk730_core::transport::DeviceInfo> {
    let found = probe_devices();
    let mut d = state.dev.lock();
    d.connected = !found.is_empty();
    d.demo = found.is_empty();
    if found.is_empty() {
        drop(d);
        push_log(
            &state,
            "list_devices: no keyboard, using demo state".to_string(),
        );
        return vec![demo_device()];
    }
    drop(d);
    push_log(
        &state,
        format!("list_devices: {} keyboard(s) connected", found.len()),
    );
    found
}

#[derive(Debug, Clone, Serialize)]
struct UsbDebug {
    hid_total: usize,
    hid_all: Vec<serde_json::Value>,
    hid_cm: Vec<serde_json::Value>,
    rusb_cm: Vec<serde_json::Value>,
    rusb_error: String,
    hint: String,
}

fn short_str(s: &str, n: usize) -> String {
    let mut t = s.replace(['\n', '\r'], " ");
    if t.len() > n {
        t.truncate(n);
    }
    t
}

#[tauri::command]
fn debug_usb(state: State<AppState>) -> UsbDebug {
    let mut hid_total = 0usize;
    let mut hid_all = Vec::new();
    let mut hid_cm = Vec::new();
    if let Ok(api) = hidapi::HidApi::new() {
        for dev in api.device_list() {
            hid_total += 1;
            // Full list (capped) so an unexpected VID/PID can be spotted.
            if hid_all.len() < 40 {
                hid_all.push(serde_json::json!({
                    "vid": format!("{:04x}", dev.vendor_id()),
                    "pid": format!("{:04x}", dev.product_id()),
                    "interface": dev.interface_number(),
                    "up": dev.usage_page(),
                    "usage": dev.usage(),
                    "product": short_str(dev.product_string().unwrap_or(""), 48),
                    "mfr": short_str(dev.manufacturer_string().unwrap_or(""), 32),
                }));
            }
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
        hid_all,
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

fn hid_send_many(
    state: &State<AppState>,
    payloads: &[Vec<u8>],
    label: &str,
) -> Result<String, CoreError> {
    let _transport = state.transport.lock();
    let started = std::time::Instant::now();
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
        CoreError::from(
            "keyboard control interface not found — check cable and close other lighting apps",
        )
    })?;
    let api = hidapi::HidApi::new().map_err(|e| CoreError::from(format!("hid init: {}", e)))?;
    let cpath = std::ffi::CString::new(path.trim_matches('"').trim_matches('\0'))
        .map_err(|e| CoreError::from(format!("bad device path: {}", e)))?;
    let dev = api.open_path(&cpath).map_err(|e| {
        CoreError::from(format!("open keyboard: {} (close Portal / other apps)", e))
    })?;
    let mut last = short.clone();
    for p in payloads {
        let mut report = [0u8; 65];
        let n = p.len().min(64);
        report[1..1 + n].copy_from_slice(&p[..n]);
        let written = dev
            .write(&report)
            .map_err(|e| CoreError::from(format!("write failed: {}", e)))?;
        if written != report.len() {
            return Err(CoreError::from(format!(
                "short HID write: {} of {} bytes",
                written,
                report.len()
            )));
        }
        let mut resp = [0u8; 65];
        let _ = dev.read_timeout(&mut resp, 25);
        last = mk730_core::proto::hexv(p);
    }
    push_log(
        state,
        format!(
            "{} ({} packets, {} ms)",
            label,
            payloads.len(),
            started.elapsed().as_millis()
        ),
    );
    Ok(last)
}

fn hid_send(state: &State<AppState>, payload: &[u8], label: &str) -> Result<String, CoreError> {
    hid_send_many(state, &[payload.to_vec()], label)
}

fn v2_init_packets(hardware: &HardwareState) -> Vec<Vec<u8>> {
    if hardware.initialized {
        return Vec::new();
    }
    let mut packets = mk730_core::proto::v2::init_mk730();
    packets.push(mk730_core::proto::v2::set_led_control(true));
    packets
}

fn v2_push_map(state: &State<AppState>) -> Result<String, CoreError> {
    let mut hardware = state.hardware.lock();
    let map = *state.colors.lock();
    let mut packets = v2_init_packets(&hardware);
    if hardware.initialized && hardware.mode != LightingMode::Direct {
        packets.extend(mk730_core::proto::v2::effect_sequence(
            mk730_core::proto::v2::DIRECT,
            3,
            255,
            0,
            [0, 0, 0],
            [0, 0, 0],
        ));
    }
    packets.extend(mk730_core::proto::v2::direct_packets(&map));
    let result = match hid_send_many(state, &packets, "paint") {
        Ok(result) => result,
        Err(error) => {
            hardware.initialized = false;
            hardware.mode = LightingMode::Unknown;
            return Err(error);
        }
    };
    hardware.initialized = true;
    hardware.mode = LightingMode::Direct;
    Ok(result)
}

fn v2_apply_effect(
    state: &State<AppState>,
    effect: u8,
    level: u8,
    brightness: u8,
    dir: u8,
    c1: [u8; 3],
    c2: [u8; 3],
) -> Result<String, CoreError> {
    let mut hardware = state.hardware.lock();
    let mut packets = v2_init_packets(&hardware);
    packets.extend(mk730_core::proto::v2::effect_sequence(
        effect, level, brightness, dir, c1, c2,
    ));
    let result = match hid_send_many(state, &packets, "effect") {
        Ok(result) => result,
        Err(error) => {
            hardware.initialized = false;
            hardware.mode = LightingMode::Unknown;
            return Err(error);
        }
    };
    hardware.initialized = true;
    hardware.mode = LightingMode::Effect;
    Ok(result)
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
    let mut hardware = state.hardware.lock();
    let h = hid_send(&state, &payload, "mode")?;
    hardware.mode = LightingMode::Unknown;
    state.dev.lock().mode = mode;
    Ok(h)
}

#[tauri::command]
fn set_full_color(state: State<AppState>, r: u8, g: u8, b: u8) -> Result<String, CoreError> {
    *state.colors.lock() = [[r, g, b]; 255];
    v2_push_map(&state)
}

#[tauri::command]
fn set_key_color(
    state: State<AppState>,
    id: u16,
    r: u8,
    g: u8,
    b: u8,
) -> Result<String, CoreError> {
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
    v2_apply_effect(&state, eid, 3, 255, 0, [124, 58, 237], [0, 0, 0])
}

#[tauri::command]
fn set_effect_params(state: State<AppState>, p: EffectParamsIn) -> Result<String, CoreError> {
    let level = p.level.clamp(1, 5);
    v2_apply_effect(&state, p.eid, level, p.brightness, p.dir, p.c1, p.c2)
}

#[tauri::command]
fn set_active_profile(state: State<AppState>, id: u8) -> Result<String, CoreError> {
    if id > 4 {
        return Err(CoreError::from("profile id 0..4"));
    }
    let mut hardware = state.hardware.lock();
    let h = hid_send(&state, &[0x51, 0x00, 0x00, 0x00, id], "profile")?;
    hardware.mode = LightingMode::Unknown;
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
    if m.events.is_empty() {
        return Err(CoreError::from("macro has no key steps"));
    }
    if m.events.len() > 2000 {
        return Err(CoreError::from("macro is limited to 2000 key steps"));
    }
    if macro_trigger_vk(&m.trigger).is_none() {
        return Err(CoreError::from(
            "trigger must be a supported key such as F6, A, or 1",
        ));
    }
    // Always succeeds locally; firmware sync explicitly pending RE.
    let mut map = state.macros.lock();
    if map
        .values()
        .any(|old| old.id != m.id && old.trigger.eq_ignore_ascii_case(&m.trigger))
    {
        return Err(CoreError::from(
            "that trigger key is already assigned to another macro",
        ));
    }
    let mut m2 = m.clone();
    m2.fw_state = "global_hotkey".to_string();
    map.insert(m2.id.clone(), m2);
    push_log(
        &state,
        format!("macro saved with PC hotkey {}: {}", m.trigger, m.id),
    );
    drop(map);
    Ok(get_macros(state))
}

fn macro_trigger_vk(trigger: &str) -> Option<u32> {
    let key = trigger.trim().to_ascii_uppercase();
    if let Some(n) = key.strip_prefix('F').and_then(|s| s.parse::<u32>().ok()) {
        return (1..=24).contains(&n).then_some(0x70 + n - 1);
    }
    if key.len() == 1 {
        let b = key.as_bytes()[0];
        if b.is_ascii_alphabetic() {
            return Some(b as u32);
        }
        if b.is_ascii_digit() {
            return Some(b as u32);
        }
    }
    match key.as_str() {
        "SPACE" => Some(0x20),
        "TAB" => Some(0x09),
        "ENTER" => Some(0x0D),
        "ESC" | "ESCAPE" => Some(0x1B),
        _ => None,
    }
}

#[tauri::command]
fn delete_macro(state: State<AppState>, id: String) -> Vec<Macro> {
    state.macros.lock().remove(&id);
    push_log(&state, format!("macro deleted: {}", id));
    get_macros(state)
}

#[cfg(windows)]
#[repr(C)]
#[derive(Clone, Copy)]
struct NativeKeyboardInput {
    vk: u16,
    scan: u16,
    flags: u32,
    time: u32,
    extra_info: usize,
}

#[cfg(windows)]
#[repr(C)]
#[derive(Clone, Copy)]
struct NativeMouseInput {
    dx: i32,
    dy: i32,
    mouse_data: u32,
    flags: u32,
    time: u32,
    extra_info: usize,
}

#[cfg(windows)]
#[repr(C)]
#[derive(Clone, Copy)]
union NativeInputData {
    keyboard: NativeKeyboardInput,
    mouse: NativeMouseInput,
}

#[cfg(windows)]
#[repr(C)]
#[derive(Clone, Copy)]
struct NativeInput {
    kind: u32,
    data: NativeInputData,
}

#[cfg(windows)]
#[link(name = "user32")]
extern "system" {
    fn SendInput(count: u32, inputs: *const NativeInput, size: i32) -> u32;
    fn RegisterHotKey(hwnd: *mut std::ffi::c_void, id: i32, modifiers: u32, vk: u32) -> i32;
    fn UnregisterHotKey(hwnd: *mut std::ffi::c_void, id: i32) -> i32;
    fn PeekMessageW(
        msg: *mut NativeMessage,
        hwnd: *mut std::ffi::c_void,
        min: u32,
        max: u32,
        remove: u32,
    ) -> i32;
}

#[cfg(windows)]
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct NativePoint {
    x: i32,
    y: i32,
}

#[cfg(windows)]
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct NativeMessage {
    hwnd: *mut std::ffi::c_void,
    message: u32,
    wparam: usize,
    lparam: isize,
    time: u32,
    point: NativePoint,
    private: u32,
}

#[cfg(windows)]
fn send_key_input(vk: u16, key_up: bool) -> Result<(), CoreError> {
    const KEYBOARD: u32 = 1;
    const KEY_UP: u32 = 0x0002;
    let input = NativeInput {
        kind: KEYBOARD,
        data: NativeInputData {
            keyboard: NativeKeyboardInput {
                vk,
                scan: 0,
                flags: if key_up { KEY_UP } else { 0 },
                time: 0,
                extra_info: 0,
            },
        },
    };
    let sent = unsafe { SendInput(1, &input, std::mem::size_of::<NativeInput>() as i32) };
    if sent != 1 {
        return Err(CoreError::from("Windows did not accept macro key input"));
    }
    Ok(())
}

#[cfg(windows)]
fn macro_hid_vk(hid: u8) -> Option<u16> {
    Some(match hid {
        0x04..=0x1d => 0x41 + (hid as u16 - 0x04),
        0x1e..=0x26 => 0x31 + (hid as u16 - 0x1e),
        0x27 => 0x30,
        0x28 => 0x0d,
        0x29 => 0x1b,
        0x2a => 0x08,
        0x2b => 0x09,
        0x2c => 0x20,
        0x2d => 0xbd,
        0x2e => 0xbb,
        0x2f => 0xdb,
        0x30 => 0xdd,
        0x31 | 0x32 => 0xdc,
        0x33 => 0xba,
        0x34 => 0xde,
        0x35 => 0xc0,
        0x36 => 0xbc,
        0x37 => 0xbe,
        0x38 => 0xbf,
        0x3a..=0x45 => 0x70 + (hid as u16 - 0x3a),
        0x46 => 0x2c,
        0x47 => 0x91,
        0x48 => 0x13,
        0x49 => 0x2d,
        0x4a => 0x24,
        0x4b => 0x21,
        0x4c => 0x2e,
        0x4d => 0x23,
        0x4e => 0x22,
        0x4f => 0x27,
        0x50 => 0x25,
        0x51 => 0x28,
        0x52 => 0x26,
        _ => return None,
    })
}

#[cfg(windows)]
fn send_macro_events(m: &Macro) -> Result<(), CoreError> {
    if m.events.is_empty() {
        return Err(CoreError::from("macro has no key steps"));
    }
    if m.events.len() > 2000 {
        return Err(CoreError::from("macro is limited to 2000 key steps"));
    }
    for event in &m.events {
        if event.hid != 0 && macro_hid_vk(event.hid).is_none() {
            return Err(CoreError::from(format!(
                "unsupported HID key 0x{:02x}",
                event.hid
            )));
        }
    }
    let repeats = if m.repeat == 0 { 1 } else { m.repeat as usize };
    let mut held = 0u8;
    let mut held_keys = std::collections::HashSet::new();
    let modifier_vks = [0x11u16, 0x10, 0x12, 0x5b, 0x11, 0x10, 0x12, 0x5c];
    for _ in 0..repeats {
        for e in &m.events {
            if e.delay_ms > 0 {
                std::thread::sleep(std::time::Duration::from_millis(e.delay_ms.min(5000) as u64));
            }
            let next = e.modifier;
            for bit in 0..8 {
                let mask = 1 << bit;
                if held & mask == 0 && next & mask != 0 {
                    send_key_input(modifier_vks[bit], false)?;
                } else if held & mask != 0 && next & mask == 0 {
                    send_key_input(modifier_vks[bit], true)?;
                }
            }
            held = next;
            if e.hid != 0 {
                let vk = macro_hid_vk(e.hid).expect("validated macro key");
                send_key_input(vk, !e.pressed)?;
                if e.pressed {
                    held_keys.insert(vk);
                } else {
                    held_keys.remove(&vk);
                }
            }
        }
    }
    for bit in 0..8 {
        if held & (1 << bit) != 0 {
            send_key_input(modifier_vks[bit], true)?;
        }
    }
    for vk in held_keys {
        send_key_input(vk, true)?;
    }
    Ok(())
}

#[cfg(windows)]
fn start_macro_hotkeys(macros: Arc<Mutex<HashMap<String, Macro>>>, app: tauri::AppHandle) {
    std::thread::spawn(move || {
        const MOD_NOREPEAT: u32 = 0x4000;
        const PM_REMOVE: u32 = 1;
        const WM_HOTKEY: u32 = 0x0312;
        let mut registrations: HashMap<i32, (String, u32)> = HashMap::new();
        let mut signature = String::new();
        loop {
            let current = macros.lock().values().cloned().collect::<Vec<_>>();
            let next_signature = current
                .iter()
                .map(|m| format!("{}:{}", m.id, m.trigger.to_ascii_uppercase()))
                .collect::<Vec<_>>()
                .join("|");
            if next_signature != signature {
                for id in registrations.keys() {
                    unsafe {
                        UnregisterHotKey(std::ptr::null_mut(), *id);
                    }
                }
                registrations.clear();
                for (i, m) in current.iter().enumerate() {
                    if let Some(vk) = macro_trigger_vk(&m.trigger) {
                        let id = 0x7300 + i as i32;
                        if unsafe { RegisterHotKey(std::ptr::null_mut(), id, MOD_NOREPEAT, vk) }
                            != 0
                        {
                            registrations.insert(id, (m.id.clone(), vk));
                        } else {
                            file_log(&format!("macro hotkey registration failed: {}", m.trigger));
                        }
                    }
                }
                signature = next_signature;
            }
            let mut msg = NativeMessage::default();
            while unsafe { PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) } != 0 {
                if msg.message == WM_HOTKEY {
                    if let Some((macro_id, _)) = registrations.get(&(msg.wparam as i32)).cloned() {
                        let m = macros.lock().get(&macro_id).cloned();
                        if let Some(m) = m {
                            if let Some(w) = app.get_webview_window("main") {
                                let _ = w.minimize();
                                std::thread::sleep(std::time::Duration::from_millis(180));
                            }
                            if let Err(e) = send_macro_events(&m) {
                                file_log(&format!("macro {} failed: {}", m.name, e));
                            }
                        }
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    });
}

#[tauri::command]
fn run_macro(
    window: tauri::WebviewWindow,
    state: State<AppState>,
    id: String,
) -> Result<String, CoreError> {
    let m = state
        .macros
        .lock()
        .get(&id)
        .cloned()
        .ok_or_else(|| CoreError::from("macro not found"))?;
    let _ = window.minimize();
    std::thread::sleep(std::time::Duration::from_millis(180));
    #[cfg(windows)]
    let result = send_macro_events(&m);
    #[cfg(not(windows))]
    let result: Result<(), CoreError> = Err(CoreError::from(
        "macro playback is currently supported on Windows",
    ));
    let _ = window.unminimize();
    result?;
    Ok(format!("ran {}", m.name))
}

#[cfg(windows)]
fn send_unicode_text(text: &str) -> Result<(), CoreError> {
    const KEYBOARD: u32 = 1;
    const UNICODE: u32 = 0x0004;
    const KEY_UP: u32 = 0x0002;
    let mut inputs = Vec::with_capacity(text.encode_utf16().count() * 2);
    for unit in text.encode_utf16() {
        for flags in [UNICODE, UNICODE | KEY_UP] {
            inputs.push(NativeInput {
                kind: KEYBOARD,
                data: NativeInputData {
                    keyboard: NativeKeyboardInput {
                        vk: 0,
                        scan: unit,
                        flags,
                        time: 0,
                        extra_info: 0,
                    },
                },
            });
        }
    }
    let sent = unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_ptr(),
            std::mem::size_of::<NativeInput>() as i32,
        )
    };
    if sent != inputs.len() as u32 {
        return Err(CoreError::from(format!(
            "Windows accepted {sent} of {} text input events",
            inputs.len()
        )));
    }
    Ok(())
}

#[tauri::command]
fn type_text_now(window: tauri::WebviewWindow, text: String) -> Result<String, CoreError> {
    if text.is_empty() {
        return Err(CoreError::from("enter text to type first"));
    }
    if text.chars().count() > 5000 {
        return Err(CoreError::from("text is limited to 5000 characters"));
    }
    window
        .minimize()
        .map_err(|e| CoreError::from(format!("could not minimize app: {e}")))?;
    std::thread::sleep(std::time::Duration::from_millis(200));
    #[cfg(windows)]
    let result = send_unicode_text(&text);
    #[cfg(not(windows))]
    let result: Result<(), CoreError> =
        Err(CoreError::from("text typing is only supported on Windows"));
    let _ = window.unminimize();
    result?;
    Ok(format!("typed {} characters", text.chars().count()))
}

#[tauri::command]
fn apply_profile_to_device(state: State<AppState>, p: Profile) -> Result<String, CoreError> {
    // Select profile, apply its effect, and save as one uninterrupted transaction.
    let mut hardware = state.hardware.lock();
    let mut packets = v2_init_packets(&hardware);
    packets.push(vec![0x51, 0x00, 0x00, 0x00, p.id]);
    if p.effect_id == mk730_core::proto::v2::DIRECT {
        let mut map = [[0u8; 3]; mk730_core::proto::v2::MAX_LEDS];
        let brightness = p.brightness.min(100) as u16;
        for (i, color) in p.colormap.iter().take(map.len()).enumerate() {
            map[i] = color.map(|channel| ((channel as u16 * brightness) / 100) as u8);
        }
        *state.colors.lock() = map;
        packets.extend(mk730_core::proto::v2::direct_packets(&map));
    } else {
        let level = p.params.p1_speed.clamp(1, 5);
        let brightness = ((p.brightness.min(100) as u16 * 255) / 100) as u8;
        packets.extend(mk730_core::proto::v2::effect_sequence(
            p.effect_id,
            level,
            brightness,
            p.params.p2,
            [p.params.color1.r, p.params.color1.g, p.params.color1.b],
            [p.params.color2.r, p.params.color2.g, p.params.color2.b],
        ));
    }
    packets.push(vec![0x50, 0x55]);
    let h = match hid_send_many(&state, &packets, "profile effect") {
        Ok(result) => result,
        Err(error) => {
            hardware.initialized = false;
            hardware.mode = LightingMode::Unknown;
            return Err(error);
        }
    };
    hardware.initialized = true;
    hardware.mode = if p.effect_id == mk730_core::proto::v2::DIRECT {
        LightingMode::Direct
    } else {
        LightingMode::Effect
    };
    state.dev.lock().active_profile = p.id;
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
    let app_state = AppState {
        dev: Mutex::new(DeviceState::default()),
        profiles: Mutex::new(pmap),
        macros: Arc::new(Mutex::new(HashMap::new())),
        colors: Mutex::new([[0u8; 3]; 255]),
        hardware: Mutex::new(HardwareState {
            initialized: false,
            mode: LightingMode::Unknown,
        }),
        transport: Mutex::new(()),
    };
    // Probe once at startup (no State wrapper available yet) so the first
    // status read is accurate and failures are recorded.
    file_log("=== app start ===");
    {
        let found = probe_devices();
        let mut d = app_state.dev.lock();
        d.connected = !found.is_empty();
        d.demo = found.is_empty();
        drop(d);
        file_log(&format!(
            "startup probe: {} device(s) found, demo={}",
            found.len(),
            found.is_empty()
        ));
        for f in &found {
            file_log(&format!(
                "  found {:04x}:{:04x} {}",
                f.vid, f.pid, f.product
            ));
        }
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .setup(|app| {
            #[cfg(windows)]
            start_macro_hotkeys(app.state::<AppState>().macros.clone(), app.handle().clone());
            Ok(())
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
            delete_macro,
            run_macro,
            type_text_now
        ])
        .run(tauri::generate_context!())
        .expect("tauri run failed");
}

fn main() {
    run();
}
