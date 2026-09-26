//! TKL keymap. Linear key IDs 0..N map to matrix via calibration.
//! The firmware order for `51 a8` / `c0 02` is ascending linear ID, but the
//! physical topology must be calibrated per layout with the `record` tool
//! (see docs/RE_MACROS.md). This file ships a sane ANSI TKL default so the UI
//! works immediately; run calibration to fix any swapped keys.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyDef {
    /// Linear firmware key id (calibrate me)
    pub id: u8,
    pub label: String,
    /// grid position for UI rendering
    pub row: u8,
    pub col: u8,
    /// width in units (e.g. backspace 2.0)
    pub w: f32,
    /// extra CSS class (e.g. "iso", "accent")
    pub cls: String,
}

fn k(id: u8, label: &str, row: u8, col: u8, w: f32, cls: &str) -> KeyDef {
    KeyDef {
        id,
        label: label.to_string(),
        row,
        col,
        w,
        cls: cls.to_string(),
    }
}

/// Default 87-key ANSI TKL + 3 lightbar zones as virtual keys 200..202.
/// IDs are sequential placeholders; replace with calibrated IDs after `record`.
pub fn default_mk730_tkl() -> Vec<KeyDef> {
    let mut v: Vec<KeyDef> = Vec::new();
    let mut id: u8 = 0;
    let mut next = |label: &str, row: u8, col: u8, w: f32, cls: &str| {
        // wrap safely (87 < 255 so fine)
        let d = k(id, label, row, col, w, cls);
        id = id.wrapping_add(1);
        v.push(d);
    };

    // Row 0: Esc F1-F12 PrtSc ScrLk Pause
    next("ESC", 0, 0, 1.0, "");
    next("F1", 0, 2, 1.0, "");
    next("F2", 0, 3, 1.0, "");
    next("F3", 0, 4, 1.0, "");
    next("F4", 0, 5, 1.0, "");
    next("F5", 0, 7, 1.0, "");
    next("F6", 0, 8, 1.0, "");
    next("F7", 0, 9, 1.0, "");
    next("F8", 0, 10, 1.0, "");
    next("F9", 0, 12, 1.0, "");
    next("F10", 0, 13, 1.0, "");
    next("F11", 0, 14, 1.0, "");
    next("F12", 0, 15, 1.0, "");
    next("Prt", 0, 16, 1.0, "mini");
    next("Scr", 0, 17, 1.0, "mini");
    next("Pse", 0, 18, 1.0, "mini");

    // Row 1: ` 1..0 - = Backspace Ins Home PgUp
    let r1 = ["`", "1", "2", "3", "4", "5", "6", "7", "8", "9", "0", "-", "="];
    for (i, l) in r1.iter().enumerate() {
        next(l, 1, i as u8, 1.0, "");
    }
    next("⌫", 1, 13, 2.0, "");
    next("Ins", 1, 16, 1.0, "mini");
    next("Hom", 1, 17, 1.0, "mini");
    next("PgU", 1, 18, 1.0, "mini");

    // Row 2: Tab Q..P [ ] \ Del End PgDn
    next("Tab", 2, 0, 1.5, "");
    for (i, l) in ["Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P"].iter().enumerate() {
        next(l, 2, (i + 2) as u8, 1.0, "");
    }
    next("[", 2, 12, 1.0, "");
    next("]", 2, 13, 1.0, "");
    next("\\", 2, 14, 1.5, "");
    next("Del", 2, 16, 1.0, "mini");
    next("End", 2, 17, 1.0, "mini");
    next("PgD", 2, 18, 1.0, "mini");

    // Row 3: Caps A..L ; ' Enter
    next("Caps", 3, 0, 1.75, "");
    for (i, l) in ["A", "S", "D", "F", "G", "H", "J", "K", "L"].iter().enumerate() {
        next(l, 3, (i + 2) as u8, 1.0, "");
    }
    next(";", 3, 11, 1.0, "");
    next("'", 3, 12, 1.0, "");
    next("Enter", 3, 13, 2.25, "");

    // Row 4: Shift Z..M , . / Shift Up
    next("Shift", 4, 0, 2.25, "");
    for (i, l) in ["Z", "X", "C", "V", "B", "N", "M"].iter().enumerate() {
        next(l, 4, (i + 2) as u8, 1.0, "");
    }
    next(",", 4, 9, 1.0, "");
    next(".", 4, 10, 1.0, "");
    next("/", 4, 11, 1.0, "");
    next("Shift", 4, 12, 2.75, "");
    next("▲", 4, 17, 1.0, "mini");

    // Row 5: Ctrl Win Alt Space Alt FN Menu Left Down Right
    next("Ctrl", 5, 0, 1.25, "");
    next("Win", 5, 1, 1.25, "");
    next("Alt", 5, 2, 1.25, "");
    next("Space", 5, 3, 6.25, "");
    next("Alt", 5, 10, 1.25, "");
    next("FN", 5, 11, 1.25, "accent");
    next("Menu", 5, 12, 1.25, "");
    next("◀", 5, 16, 1.0, "mini");
    next("▼", 5, 17, 1.0, "mini");
    next("▶", 5, 18, 1.0, "mini");

    // Virtual lightbar zones
    v.push(k(200, "BAR-L", 6, 0, 4.0, "bar"));
    v.push(k(201, "BAR-F", 6, 5, 6.0, "bar"));
    v.push(k(202, "BAR-R", 6, 12, 4.0, "bar"));

    v
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn has_tkl_count() {
        let m = default_mk730_tkl();
        assert!(m.len() >= 87);
    }
}
