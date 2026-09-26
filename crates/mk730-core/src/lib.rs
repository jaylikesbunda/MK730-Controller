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

/// Known Cooler Master USB vendor ID (OpenRGB: COOLERMASTER_VID).
pub const CM_VID: u16 = 0x2516;
/// Legacy / alternate VID seen in older docs. Kept for scanning fallback.
pub const CM_VID_ALT: u16 = 0x2512;

/// Candidate PIDs (V2 family, OpenRGB CMKeyboardDevices.h).
/// MK730 = 0x008F, MK750 = 0x0067. Others included so sibling boards also show up.
pub const CANDIDATE_PIDS: &[u16] = &[
    0x008F, // MK730
    0x0067, // MK750
    0x009F, 0x0147, 0x0145, 0x007F, // CK530 / CK550 V2 / CK552 V2
    0x0089, 0x008D, // SK630 / SK650
    0x015D, 0x01AB, 0x0149, 0x014B, // SK652 / SK653 / SK622
    0x0157, 0x0159, // SK620
    0x0069, // MK850
];

/// All VIDs we scan for.
pub const SUPPORTED_VIDS: &[u16] = &[CM_VID, CM_VID_ALT];

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
