//! Packet builders: all payloads are 64 bytes, zero-padded.
//! Layouts follow libcmmk PROTOCOL.md.

use serde::{Deserialize, Serialize};

/// Control mode (41 xx)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ControlMode {
    Firmware = 0x00,
    Effect = 0x01,
    Manual = 0x02,
    Profile = 0x03,
}

/// Effect IDs (51/52 28, 2c)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum EffectId {
    FullyLit = 0x00,
    Breathe = 0x01,
    Cycle = 0x02,
    Single = 0x03,
    Wave = 0x04,
    Ripple = 0x05,
    Cross = 0x06,
    Raindrops = 0x07,
    Stars = 0x08,
    Snake = 0x09,
    Customized = 0x0A,
    Multilayer = 0xE0,
    Off = 0xFE,
}

impl EffectId {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0x00 => Some(Self::FullyLit),
            0x01 => Some(Self::Breathe),
            0x02 => Some(Self::Cycle),
            0x03 => Some(Self::Single),
            0x04 => Some(Self::Wave),
            0x05 => Some(Self::Ripple),
            0x06 => Some(Self::Cross),
            0x07 => Some(Self::Raindrops),
            0x08 => Some(Self::Stars),
            0x09 => Some(Self::Snake),
            0x0A => Some(Self::Customized),
            0xE0 => Some(Self::Multilayer),
            0xFE => Some(Self::Off),
            _ => None,
        }
    }

    pub fn all() -> Vec<(u8, &'static str)> {
        vec![
            (0x00, "Fully lit"),
            (0x01, "Breathe"),
            (0x02, "Color cycle"),
            (0x03, "Single key"),
            (0x04, "Wave"),
            (0x05, "Ripple"),
            (0x06, "Cross"),
            (0x07, "Raindrops"),
            (0x08, "Stars"),
            (0x09, "Snake"),
            (0x0A, "Customized"),
            (0xE0, "Multilayer"),
            (0xFE, "Off"),
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum WaveDirection {
    LeftToRight = 0x00,
    BackToFront = 0x02,
    RightToLeft = 0x04,
    FrontToBack = 0x06,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectParams {
    /// Speed: ~10 (fast) .. 50 (slow). Portal uses fixed steps, FW interpolates.
    pub p1_speed: u8,
    /// Meaning depends on effect: wave direction (04) or ripple mode (05: 0x00=color, 0x80=random).
    pub p2: u8,
    /// Intensity/interval for 07/08.
    pub p3: u8,
    pub color1: Rgb,
    pub color2: Rgb,
    /// 0x00 normal, 0x01 multilayer
    pub multilayer: u8,
}

impl Default for EffectParams {
    fn default() -> Self {
        Self {
            p1_speed: 32,
            p2: 0x00,
            p3: 32,
            color1: Rgb::new(124, 58, 237),
            color2: Rgb::new(0, 0, 0),
            multilayer: 0x00,
        }
    }
}

fn blank() -> [u8; 64] {
    [0u8; 64]
}

/// 41 xx: change control mode
pub fn build_set_mode(mode: ControlMode) -> [u8; 64] {
    let mut p = blank();
    p[0] = 0x41;
    p[1] = mode as u8;
    p
}

/// 51 00 / 52 00: set/get active profile. Payload: 00 00 <id>
pub fn build_set_active_profile(profile_id: u8) -> [u8; 64] {
    let mut p = blank();
    p[0] = 0x51;
    p[1] = 0x00;
    p[2] = 0x00;
    p[3] = 0x00;
    p[4] = profile_id;
    p
}

pub fn build_get_active_profile() -> [u8; 64] {
    let mut p = blank();
    p[0] = 0x52;
    p[1] = 0x00;
    p
}

/// 50 55: save current profile to firmware
pub fn build_save_profile() -> [u8; 64] {
    let mut p = blank();
    p[0] = 0x50;
    p[1] = 0x55;
    p
}

/// 51 28: set active effect. Payload: 00 00 <eid>
pub fn build_set_effect(eid: u8) -> [u8; 64] {
    let mut p = blank();
    p[0] = 0x51;
    p[1] = 0x28;
    p[2] = 0x00;
    p[3] = 0x00;
    p[4] = eid;
    p
}

pub fn build_get_effect() -> [u8; 64] {
    let mut p = blank();
    p[0] = 0x52;
    p[1] = 0x28;
    p
}

/// 51 2c: set effect params.
/// Payload: <ml> 00 <eid> <p1> <p2> <p3> <r1><g1><b1> <r2><g2><b2> [ff...]
pub fn build_set_effect_params(eid: u8, params: &EffectParams) -> [u8; 64] {
    let mut p = [0xFFu8; 64];
    p[0] = 0x51;
    p[1] = 0x2C;
    p[2] = params.multilayer;
    p[3] = 0x00;
    p[4] = eid;
    p[5] = params.p1_speed;
    p[6] = params.p2;
    p[7] = params.p3;
    p[8] = params.color1.r;
    p[9] = params.color1.g;
    p[10] = params.color1.b;
    p[11] = params.color2.r;
    p[12] = params.color2.g;
    p[13] = params.color2.b;
    p
}

pub fn build_get_effect_params(multilayer: u8, eid: u8) -> [u8; 64] {
    let mut p = blank();
    p[0] = 0x52;
    p[1] = 0x2C;
    p[2] = multilayer;
    p[3] = 0x00;
    p[4] = eid;
    p
}

/// c0 00: entire keyboard single color. Payload: 00 00 <r><g><b>
pub fn build_manual_full_color(r: u8, g: u8, b: u8) -> [u8; 64] {
    let mut p = blank();
    p[0] = 0xC0;
    p[1] = 0x00;
    p[2] = 0x00;
    p[3] = 0x00;
    p[4] = r;
    p[5] = g;
    p[6] = b;
    p
}

/// c0 01: single key. Payload: 01 00 <key_id> <r><g><b>
pub fn build_manual_key(key_id: u8, r: u8, g: u8, b: u8) -> [u8; 64] {
    let mut p = blank();
    p[0] = 0xC0;
    p[1] = 0x01;
    p[2] = 0x01;
    p[3] = 0x00;
    p[4] = key_id;
    p[5] = r;
    p[6] = g;
    p[7] = b;
    p
}

/// 51 a8 / c0 02: 8 packets of 16 RGB triplets.
/// o1 = 2*i for i-th packet. Returns packets ready to send in order.
pub fn build_colormap_packets(colors_linear: &[[u8; 3]], manual: bool) -> Vec<[u8; 64]> {
    let mut out = Vec::new();
    for i in 0..8 {
        let mut p = blank();
        if manual {
            p[0] = 0xC0;
            p[1] = 0x02;
        } else {
            p[0] = 0x51;
            p[1] = 0xA8;
        }
        p[2] = (i * 2) as u8;
        p[3] = 0x00;
        for k in 0..16 {
            let idx = i * 16 + k;
            let (r, g, b) = if idx < colors_linear.len() {
                let c = colors_linear[idx];
                (c[0], c[1], c[2])
            } else {
                (0, 0, 0)
            };
            p[4 + k * 3] = r;
            p[5 + k * 3] = g;
            p[6 + k * 3] = b;
        }
        // last byte of 8th packet region overlaps 64B boundary; truncate safely
        // (4 + 48 = 52 bytes used, rest stays 0)
        out.push(p);
    }
    out
}

/// Format packet as hex string for logs / USB capture comparison.
pub fn hex(packet: &[u8; 64]) -> String {
    packet
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn hexv(packet: &[u8]) -> String {
    packet
        .iter()
        .take(24)
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

/// V2 protocol (MK730/MK750/CK/SK — OpenRGB CMKeyboardV2Controller).
/// Transport is HID (hid_write 65B with leading report ID 0), usage page
/// 0xFF00, interface 1. Payloads below exclude the leading report byte.
pub mod v2 {
    /// V2 effect IDs (CMKeyboardDevices.h cm_keyboard_effect_type).
    pub const DIRECT: u8 = 1;
    pub const STATIC: u8 = 4;
    pub const BREATHE: u8 = 5;
    pub const CYCLE: u8 = 6;
    pub const WAVE: u8 = 7;
    pub const RIPPLE: u8 = 8;
    pub const CROSS: u8 = 9;
    pub const RAINDROPS: u8 = 10;
    pub const STARS: u8 = 11;
    pub const CUSTOMIZED: u8 = 13;
    pub const REACTIVE_FADE: u8 = 16;
    pub const HEARTBEAT: u8 = 19;
    pub const FIREBALL: u8 = 20;
    pub const SNOW: u8 = 21;
    pub const CIRCLE_SPECTRUM: u8 = 22;
    pub const WATER_RIPPLE: u8 = 23;
    pub const OFF: u8 = 24;

    pub const N_LEDS: u8 = 0xC1;
    pub const MAX_LEDS: usize = 255;

    pub fn all_effects() -> Vec<(u8, &'static str)> {
        vec![
            (DIRECT, "Custom paint"),
            (STATIC, "Steady"),
            (BREATHE, "Breathing"),
            (CYCLE, "Color cycle"),
            (WAVE, "Wave"),
            (RIPPLE, "Ripple"),
            (CROSS, "Crosshair"),
            (RAINDROPS, "Rain"),
            (STARS, "Stars"),
            (SNOW, "Snow"),
            (FIREBALL, "Fireball"),
            (HEARTBEAT, "Heartbeat"),
            (WATER_RIPPLE, "Water ripple"),
            (REACTIVE_FADE, "Reactive fade"),
            (CIRCLE_SPECTRUM, "Circle spectrum"),
            (CUSTOMIZED, "Customized"),
            (OFF, "Off"),
        ]
    }

    /// MK730 init sequence: profile 0x05 + magic 0x0A.
    pub fn init_mk730() -> Vec<Vec<u8>> {
        vec![
            vec![0x51, 0x00, 0x00, 0x00, 0x05],
            vec![0x56, 0x81, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
                 0x0A, 0x00, 0x00, 0x00, 0xBB, 0xBB, 0xBB, 0xBB],
        ]
    }

    pub fn set_led_control(manual: bool) -> Vec<u8> {
        vec![0x41, if manual { 0x05 } else { 0x00 }]
    }

    pub fn custom_mode() -> Vec<Vec<u8>> {
        vec![vec![0x41, 0x80], vec![0x52]]
    }

    pub fn apply(on: bool) -> Vec<u8> {
        vec![0x51, 0x28, 0x00, 0x00, if on { 0xFF } else { 0x10 }]
    }

    fn effect_mode_packet(effect: u8) -> Vec<u8> {
        match effect {
            STATIC | DIRECT | CYCLE | BREATHE | CIRCLE_SPECTRUM => vec![
                0x56, 0x81, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
                0x01, 0x00, 0x00, 0x00, 0x88, 0x88, 0x88, 0x88],
            WAVE => vec![
                0x56, 0x81, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
                0x01, 0x00, 0x00, 0x00, 0x99, 0x99, 0x99, 0x99],
            CUSTOMIZED => vec![
                0x56, 0x81, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
                0x09, 0x00, 0x00, 0x00, 0xBB, 0xBB, 0xBB, 0xBB],
            _ => vec![
                0x56, 0x81, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00,
                0x02, 0x00, 0x00, 0x00, 0x88, 0x88, 0x88, 0x88],
        }
    }

    /// Speed tables from OpenRGB _UpdateSpeed. level 1..5.
    pub fn speed_pair(effect: u8, level: u8, dir_right: bool) -> (u8, u8) {
        let l = level.clamp(1, 5) as usize - 1;
        let pick = |a: [u8; 5], b: [u8; 5]| (a[l], b[l]);
        match effect {
            CROSS | REACTIVE_FADE => pick([0x17,0x0E,0x0B,0x0A,0x04],[0x01,0x01,0x02,0x05,0x04]),
            WAVE => pick([0x17,0x0D,0x07,0x09,0x08],[0x01,0x04,0x06,0x0C,0x11]),
            BREATHE => pick([0x08,0x0A,0x0C,0x07,0x09],[0x01,0x02,0x04,0x04,0x09]),
            CIRCLE_SPECTRUM => if dir_right {
                pick([0x00,0x01,0x02,0x03,0x04],[0x0C,0x08,0x08,0x04,0x00])
            } else {
                pick([0xFF,0xFE,0xFD,0xFC,0xFC],[0x04,0x08,0x08,0x0C,0x00])
            },
            CYCLE => ([0x10,0x0C,0x08,0x04,0x00][l], 0x00),
            RIPPLE | WATER_RIPPLE => ([0x36,0x18,0x0C,0x06,0x02][l], 0x10),
            RAINDROPS | SNOW => pick([0x0B,0x08,0x05,0x02,0x00],[0x08,0x18,0x30,0x38,0x40]),
            STARS => pick([0x17,0x0E,0x08,0x0A,0x0A],[0x01,0x01,0x01,0x02,0x04]),
            HEARTBEAT | FIREBALL => ([0x01,0x02,0x03,0x05,0x09][l], 0x00),
            _ => (0x08, 0x01),
        }
    }

    /// Build full V2 effect sequence. level 1..5, brightness 0..255,
    /// dir_code 0x00=right 0x04=left 0x02=down 0x06=up.
    pub fn effect_sequence(
        effect: u8,
        level: u8,
        brightness: u8,
        dir_code: u8,
        c1: [u8; 3],
        c2: [u8; 3],
    ) -> Vec<Vec<u8>> {
        let mut out: Vec<Vec<u8>> = Vec::new();
        for p in custom_mode() { out.push(p); }
        out.push(effect_mode_packet(effect));
        let (s1, _s2) = speed_pair(effect, level, dir_code == 0x00);
        let s2 = speed_pair(effect, level, dir_code == 0x00).1;
        match effect {
            OFF => {}
            DIRECT => {}
            STATIC => {
                out.push(vec![
                    0x56, 0x83, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
                    0x00, 0x01, 0x00, N_LEDS, 0x00, 0x00, 0x00, 0x00,
                    c1[0], c1[1], c1[2], brightness, 0x00, 0x00, 0x00, 0x00]);
            }
            WAVE => {
                out.push(vec![
                    0x56, 0x83, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
                    0x00, 0x32, 0x00, N_LEDS, s1, 0x00, 0x00, 0x00,
                    0xFF, 0xFF, 0xFF, brightness, 0x00, dir_code, s2, 0x00,
                    0x00, 0x04, 0x08]);
            }
            CYCLE => {
                out.push(vec![
                    0x56, 0x83, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
                    0x00, 0x31, 0x00, N_LEDS, s1, 0x00, 0x00, 0x00,
                    0x40, 0x00, 0xFF, brightness, 0x00, 0x00, 0x03, 0x00]);
            }
            BREATHE => {
                out.push(vec![
                    0x56, 0x83, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
                    0x00, 0x30, 0x00, N_LEDS, s1, 0x00, 0x00, 0x00,
                    c1[0], c1[1], c1[2], brightness, 0x01, 0x00, s2, 0x00]);
            }
            STARS | RAINDROPS | SNOW | CROSS | RIPPLE
            | REACTIVE_FADE | HEARTBEAT | FIREBALL | WATER_RIPPLE => {
                let (b0, b1, b2, b3): (u8, u8, u8, u8) = match effect {
                    STARS => (0x00, 0x40, 0x00, 0x80),
                    RAINDROPS | SNOW => (0x81, 0x40, 0x00, 0x80),
                    CROSS | REACTIVE_FADE => (0x00, 0x80, 0x00, 0x80),
                    RIPPLE | WATER_RIPPLE => (0x01, 0x82, 0x00, 0x80),
                    HEARTBEAT | FIREBALL => (0x20, 0xA0, 0x00, 0x80),
                    _ => (0x00, 0x80, 0x00, 0x80),
                };
                let mut p = vec![
                    0x56, 0x83, 0x00, 0x00, 0x0D, 0x00, 0x0D, 0x00,
                    0x03, 0x00, 0x00, 0x00, 0x07, 0x00, 0x01, 0x00,
                    0x00, 0x01, 0x00, N_LEDS, 0x00, 0x00, 0x00, 0x00,
                    c2[0], c2[1], c2[2], brightness, 0x00, 0x00, 0x00, 0x00,
                    b0, b1, b2, b3, s1, 0x10, 0x00, 0x00,
                    c1[0], c1[1], c1[2], brightness,
                ];
                p.extend_from_slice(&[0x00, 0x00, 0x01, 0x00]);
                while p.len() < 64 { p.push(0xFF); }
                out.push(p);
                let mut t = vec![0x56, 0x83, 0x01, 0x00];
                t.extend(vec![0xFF; 52]);
                out.push(t);
            }
            _ => {}
        }
        out.push(vec![0x41, 0x80]);
        out.push(apply(effect != OFF));
        out
    }

    /// Direct per-key colormap push (V2). map255 indexed by LED value.
    pub fn direct_packets(map255: &[[u8; 3]; MAX_LEDS]) -> Vec<Vec<u8>> {
        let mut flat = vec![0u8; MAX_LEDS * 3];
        for i in 0..MAX_LEDS {
            flat[i * 3] = map255[i][0];
            flat[i * 3 + 1] = map255[i][1];
            flat[i * 3 + 2] = map255[i][2];
        }
        let mut out = Vec::new();
        let mut p0 = vec![
            0x56, 0x83, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
            0x80, 0x01, 0x00, N_LEDS, 0x00, 0x00, 0x00, 0x00,
            0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00];
        p0.extend_from_slice(&flat[0..40]);
        out.push(p0);
        let mut idx = 40usize;
        for i in 1..10 {
            let mut p = vec![0x56, 0x83, i as u8, 0x00];
            let end = (idx + 60).min(flat.len());
            p.extend_from_slice(&flat[idx..end]);
            while p.len() < 64 { p.push(0); }
            out.push(p);
            idx = end;
        }
        out.push(apply(true));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_packet_shape() {
        let p = build_set_mode(ControlMode::Manual);
        assert_eq!((p[0], p[1]), (0x41, 0x02));
        assert_eq!(p.len(), 64);
    }

    #[test]
    fn effect_packet_shape() {
        let p = build_set_effect(0x04);
        assert_eq!((p[0], p[1], p[4]), (0x51, 0x28, 0x04));
    }

    #[test]
    fn colormap_is_8_packets() {
        let colors = vec![[255, 0, 0]; 128];
        let pkts = build_colormap_packets(&colors, true);
        assert_eq!(pkts.len(), 8);
        assert_eq!((pkts[0][0], pkts[0][1], pkts[0][2]), (0xC0, 0x02, 0));
        assert_eq!(pkts[1][2], 2);
    }
}
