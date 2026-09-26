//! MK730 keymap with real V2 firmware LED values (OpenRGB mk730_keymap).
//! V2 direct mode has no single-LED command — the full 0xC1-entry colormap
//! is pushed every time, indexed by these LED values.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyDef {
    /// Firmware LED index for V2 `56 83` colormap
    pub id: u16,
    pub label: String,
    pub row: u8,
    pub col: u8,
    pub w: f32,
    pub cls: String,
}

fn k(id: u16, label: &str, row: u8, col: u8, w: f32, cls: &str) -> KeyDef {
    KeyDef {
        id,
        label: label.to_string(),
        row,
        col,
        w,
        cls: cls.to_string(),
    }
}

/// ANSI TKL with real LED values. Skips ISO-only keys.
pub fn default_mk730_tkl() -> Vec<KeyDef> {
    let mut v: Vec<KeyDef> = Vec::new();

    // Row 0
    for (i, l) in ["ESC","F1","F2","F3","F4","F5","F6","F7","F8","F9","F10","F11","F12","Prt","Scr","Pse"].iter().enumerate() {
        let led = [7,28,35,42,49,63,70,77,84,91,98,105,112,119,126,133][i] as u16;
        let col = match i { 0=>0, 1=>2, 2=>3, 3=>4, 4=>5, 5=>7, 6=>8, 7=>9, 8=>10, 9=>12, 10=>13, 11=>14, 12=>15, _=> 16+(i as u8 -13) };
        v.push(k(led, l, 0, col, 1.0, if i>=13 {"mini"} else {""}));
    }
    // Row 1
    for (i, l) in ["`","1","2","3","4","5","6","7","8","9","0","-","=","Back","Ins","Hom","PgU"].iter().enumerate() {
        let led = [8,22,29,36,43,50,57,64,71,78,85,92,99,113,120,127,134][i] as u16;
        let w = if *l=="Back" {2.0} else {1.0};
        v.push(k(led, if *l=="Back" {"⌫"} else {l}, 1, i as u8, w, if i>=14 {"mini"} else {""}));
    }
    // Row 2
    let r2 = ["Tab","Q","W","E","R","T","Y","U","I","O","P","[","]","\\","Del","End","PgD"];
    let r2v = [9,23,30,37,44,51,58,65,72,79,86,93,100,114,121,128,135];
    for (i, l) in r2.iter().enumerate() {
        let w = if *l=="Tab" {1.5} else if *l=="\\" {1.5} else {1.0};
        v.push(k(r2v[i] as u16, l, 2, i as u8, w, if i>=14 {"mini"} else {""}));
    }
    // Row 3 (skip ISO # = 108)
    let r3 = ["Caps","A","S","D","F","G","H","J","K","L",";", "'", "Enter"];
    let r3v = [10,24,31,38,45,52,59,66,73,80,87,94,115];
    for (i, l) in r3.iter().enumerate() {
        let w = if *l=="Caps" {1.75} else if *l=="Enter" {2.25} else {1.0};
        v.push(k(r3v[i] as u16, l, 3, i as u8, w, ""));
    }
    // Row 4 (skip ISO \ = 18)
    let r4 = ["Shift","Z","X","C","V","B","N","M",",",".","/","Shift","Up"];
    let r4v = [11,25,32,39,46,53,60,67,74,81,88,116,130];
    for (i, l) in r4.iter().enumerate() {
        let w = if *l=="Shift" && i==0 {2.25} else if *l=="Shift" {2.75} else {1.0};
        let col = if i>=12 {17} else {i as u8};
        v.push(k(r4v[i] as u16, if *l=="Up" {"▲"} else {l}, 4, col, w, if i==12 {"mini"} else {""}));
    }
    // Row 5
    let r5 = ["Ctrl","Win","Alt","Space","Alt","FN","Menu","Ctrl","Left","Down","Right"];
    let r5v = [12,19,26,54,82,89,96,117,124,131,138];
    for (i, l) in r5.iter().enumerate() {
        let (w, col) = match i {
            3 => (6.25, 3),
            8 => (1.0, 16), 9 => (1.0, 17), 10 => (1.0, 18),
            _ => (1.25, i as u8),
        };
        let label = match *l { "Left"=>"◀", "Down"=>"▼", "Right"=>"▶", x=>x };
        v.push(k(r5v[i] as u16, label, 5, col, w, if i>=7 {"mini"} else if *l=="FN" {"accent"} else {""}));
    }

    v
}

/// Lightbar / underglow LED groups (V2 colormap indices).
pub fn lightbar_groups() -> Vec<(String, Vec<u16>)> {
    vec![
        ("Left".to_string(), vec![1,2,3,4]),
        ("Right".to_string(), vec![141,142,143,144]),
        ("Front".to_string(), vec![13,20,27,34,41,55,62,69,76,90,104,111,118,125]),
        ("Logo".to_string(), vec![69]),
    ]
}

/// Max LEDs for V2 direct packets.
pub const V2_N_LEDS: u8 = 0xC1;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn has_tkl_count() {
        let m = default_mk730_tkl();
        assert!(m.len() >= 85);
        // spot-check real values: ESC=7, Space=54, Enter=115
        let esc = m.iter().find(|k| k.label=="ESC").unwrap();
        assert_eq!(esc.id, 7);
        let sp = m.iter().find(|k| k.label=="Space").unwrap();
        assert_eq!(sp.id, 54);
    }
}
