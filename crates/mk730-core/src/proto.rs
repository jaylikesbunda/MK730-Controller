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
