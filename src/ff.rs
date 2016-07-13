use gamepad::Button;
use std::u16::MAX as U16_MAX;
use std::f32::consts::PI;

pub use gamepad::Effect;

#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub struct EffectData {
    pub wave: Waveform,
    pub direction: Direction,
    pub period: u16,
    pub magnitude: i16,
    pub offset: i16,
    pub phase: u16,
    pub envelope: Envelope,
    pub replay: Replay,
    pub trigger: Trigger,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Waveform {
    Square,
    Triangle,
    Sine,
}

impl Default for Waveform {
    fn default() -> Self { Waveform::Sine }
}

#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub struct Direction {
    pub angle: u16,
}

impl From<f32> for Direction {
    fn from(f: f32) -> Self {
        let f = if f < 0.0 {
            0.0
        } else if f > 1.0 {
            1.0
        } else {
            f
        };
        Direction { angle: (U16_MAX as f32 * f) as u16 }
    }
}

impl From<[f32; 2]> for Direction {
    fn from(f: [f32; 2]) -> Self {
        let mut val = f[1].atan2(f[0]);
        if val.is_sign_negative() {
            val += 2.0 * PI;
        }
        (val / (2.0 * PI)).into()
    }
}

#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub struct Envelope {
    pub attack_length: u16,
    pub attack_level: u16,
    pub fade_length: u16,
    pub fade_level: u16,
}

#[derive(Copy, Clone, PartialEq, Debug, Default)]
#[repr(C)]
pub struct Replay {
    pub length: u16,
    pub delay: u16,
}

#[derive(Copy, Clone, PartialEq, Debug, Default)]
#[repr(C)]
pub struct Trigger {
    pub button: Button,
    pub interval: u16,
}
