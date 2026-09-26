//! mk730-core: pure protocol builders for Cooler Master MasterKeys family.
//!
//! Based on captured USB data documented by `chmod222/libcmmk` (LGPL-3.0,
//! PROTOCOL.md): 64-byte interrupt packets on Interface 1, OUT 0x04 / IN 0x83.
//! MK730 shares Portal software + MCU with MK750, so same family. Key IDs and
//! macro packets still need per-device calibration (see docs/RE_MACROS.md).

pub mod keymap;
pub mod macros;
pub mod profiles;
pub mod proto;
pub mod transport;

pub use keymap::{KeyDef, default_mk730_tkl};
pub use macros::{Macro, MacroEvent};
pub use profiles::Profile;
pub use proto::{EffectId, EffectParams, WaveDirection};
pub use transport::{DeviceInfo, TransportKind};

/// Known Cooler Master USB vendor ID.
pub const CM_VID: u16 = 0x2512;

/// Candidate PIDs (Portal family). MK730 PIDs vary by switch/layout;
/// confirm with `lsusb -v -d 2512:` and add yours here.
pub const CANDIDATE_PIDS: &[u16] = &[
    0x0067, // MK750 (reference)
    0x0086, 0x0087, 0x0088, // commonly seen CM keyboard range, verify
];

/// App-level error (serializable for Tauri).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CoreError {
    pub message: String,
}

impl std::fmt::Display for CoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl From<String> for CoreError {
    fn from(message: String) -> Self {
        Self { message }
    }
}

impl From<&str> for CoreError {
    fn from(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}
