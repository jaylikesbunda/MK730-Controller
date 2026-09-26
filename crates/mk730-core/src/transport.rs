//! Device transport types. Core stays system-free; actual
//! libusb/hidapi I/O lives in src-tauri. This keeps `cargo test` green on CI
//! without hardware and lets the UI run in demo mode.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub vid: u16,
    pub pid: u16,
    pub interface: u8,
    pub path: String,
    pub product: String,
    pub demo: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportKind {
    LibUsb,
    HidApi,
    Mock,
}

/// Build a demo device entry when no hardware is attached so the
/// launcher UI is still fully explorable.
pub fn demo_device() -> DeviceInfo {
    DeviceInfo {
        vid: crate::CM_VID,
        pid: 0x0067,
        interface: 1,
        path: "demo://mk730".to_string(),
        product: "MK730 (demo — no hardware)".to_string(),
        demo: true,
    }
}
