//! Macros: local model works today; firmware encoding needs RE.
//!
//! Status: Portal supports on-device macros + remap, but the USB encoding is
//! NOT in libcmmk / SDK. Until docs/RE_MACROS.md captures land, the launcher
//! stores macros locally (JSON) and marks firmware sync as `pending_capture`.
//! The UI is fully functional (record, edit, assign) so captures can be
//! attached to the exact trigger key.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroEvent {
    /// USB HID usage (e.g. 0x04 = 'a'), or 0 for delay-only event
    pub hid: u8,
    pub pressed: bool,
    /// ms since previous event
    pub delay_ms: u16,
    pub modifier: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Macro {
    pub id: String,
    pub name: String,
    /// label of trigger key, e.g. "F5" or "G1"
    pub trigger: String,
    pub events: Vec<MacroEvent>,
    pub repeat: u8, // 0 = once, 255 = toggle loop
    /// firmware sync state: "local_only" | "pending_capture" | "synced"
    pub fw_state: String,
}

impl Macro {
    pub fn new(trigger: &str) -> Self {
        Self {
            id: format!("m_{}", chrono_stamp()),
            name: format!("Macro {}", trigger),
            trigger: trigger.to_string(),
            events: Vec::new(),
            repeat: 0,
            fw_state: "local_only".to_string(),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub fn from_json(s: &str) -> Result<Self, String> {
        serde_json::from_str(s).map_err(|e| e.to_string())
    }
}

fn chrono_stamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Placeholder encoder: returns Err until RE lands.
/// Keeps the firmware path explicit instead of silently pretending to work.
pub fn encode_for_firmware(_m: &Macro) -> Result<Vec<[u8; 64]>, String> {
    Err("macro firmware encoding not yet reverse-engineered — stored locally, see docs/RE_MACROS.md".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stub_errors_clearly() {
        let m = Macro::new("F5");
        assert!(encode_for_firmware(&m).is_err());
    }
}
