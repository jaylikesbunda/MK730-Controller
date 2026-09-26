//! Profiles: P1..P5 local JSON + firmware save.
//! Firmware fields mirror `51 xx` payloads; local file adds name + colormap.

use serde::{Deserialize, Serialize};
use crate::proto::{EffectParams, Rgb};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: u8, // 0..4
    pub name: String,
    pub effect_id: u8,
    pub params: EffectParams,
    /// linear colormap, up to 128 entries [r,g,b]
    pub colormap: Vec<[u8; 3]>,
    pub brightness: u8, // 0..100 UI only
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            id: 0,
            name: "P1".to_string(),
            effect_id: 0x04,
            params: EffectParams::default(),
            colormap: vec![[124, 58, 237]; 128],
            brightness: 100,
        }
    }
}

impl Profile {
    pub fn default_set() -> Vec<Profile> {
        (0..5)
            .map(|i| Profile {
                id: i,
                name: format!("P{}", i + 1),
                ..Default::default()
            })
            .collect()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub fn from_json(s: &str) -> Result<Self, String> {
        serde_json::from_str(s).map_err(|e| e.to_string())
    }

    /// brightness-scaled color helper for UI preview
    pub fn scaled(&self, c: Rgb) -> Rgb {
        let b = self.brightness as u16;
        Rgb::new(
            ((c.r as u16 * b) / 100) as u8,
            ((c.g as u16 * b) / 100) as u8,
            ((c.b as u16 * b) / 100) as u8,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip() {
        let p = Profile::default();
        let s = p.to_json();
        let q = Profile::from_json(&s).unwrap();
        assert_eq!(q.id, 0);
    }
}
