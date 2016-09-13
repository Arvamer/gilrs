// Copyright 2016 GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

//! Force feedback module
//!
//! To use force feedback create `EffectData` struct, upload it to device using
//! [`Gamepad::add_ff_effect`](../struct.Gamepad.html) and use `play()` function or wait for trigger
//! event.
//!
//! ```rust,no_run
//! use gilrs::ff::EffectData;
//! use gilrs::Gilrs;
//!
//! let mut gilrs = Gilrs::new();
//!
//! let mut effect = EffectData::default();
//! effect.period = 1000;
//! effect.magnitude = 20000;
//! effect.replay.length = 5000;
//! effect.envelope.attack_length = 1000;
//! effect.envelope.fade_length = 1000;
//!
//! let effect_idx = gilrs.gamepad_mut(0).add_ff_effect(effect).unwrap();
//! gilrs.gamepad_mut(0).ff_effect(effect_idx).unwrap().play(1);
//! ```

use gamepad::Button;
use std::u16::MAX as U16_MAX;
use std::f32::consts::PI;
use std::error::Error as StdError;
use std::fmt;

pub use gamepad::Effect;

/// Describes wave-shaped force feedback effect that repeat itself over time.
///
/// *Borrowed* from [SDL Documentation](https://wiki.libsdl.org/SDL_HapticPeriodic):
///
/// ```text
/// button         period
/// press          |     |
///   ||      __    __    __    __    __    _
///   ||     |  |  |  |  |  |  |  |  |  |   magnitude
///   \/     |  |__|  |__|  |__|  |__|  |   _
///    -----
///       |            offset?
///     delay          phase?
///
/// -------------------------------------
///               length
/// ===================================================
///                       interval
/// ```
#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub struct EffectData {
    /// Kind of the wave
    pub wave: Waveform,
    /// Direction of the effect
    pub direction: Direction,
    /// Period of the wave in ms
    pub period: u16,
    /// Peak value
    pub magnitude: i16,
    /// Mean value of the wave
    pub offset: i16,
    /// Horizontal shift
    pub phase: u16,
    /// Envelope data
    pub envelope: Envelope,
    /// Scheduling of the effect
    pub replay: Replay,
    /// Trigger conditions
    pub trigger: Trigger,
}

/// Wave shape.
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Waveform {
    Square,
    Triangle,
    Sine,
}

impl Default for Waveform {
    fn default() -> Self {
        Waveform::Sine
    }
}

/// Direction of force feedback effect.
///
/// Angle is represented by value from 0 to u16::MAX, which map to [0, 2π].
///
/// ```
/// use std::u16::MAX;
/// use std::f32::consts::PI;
/// # use gilrs::ff::Direction;
///
/// let direction = Direction { angle: MAX / 2 };
/// assert_eq!(direction, Direction::from_radians(PI));
/// assert_eq!(direction, Direction::from_vector([-1.0, 0.0]));
/// ```
#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub struct Direction {
    pub angle: u16,
}

impl Direction {
    pub fn from_radians(ang: f32) -> Self {
        let mut ang = ang % (2.0 * PI);
        if ang < 0.0 {
            ang = 2.0 * PI - ang
        };
        ang /= 2.0 * PI;
        Direction { angle: (U16_MAX as f32 * ang) as u16 }
    }

    pub fn from_vector(vec: [f32; 2]) -> Self {
        vec.into()
    }
}

impl From<[f32; 2]> for Direction {
    fn from(f: [f32; 2]) -> Self {
        let mut val = f[1].atan2(f[0]);
        if val.is_sign_negative() {
            val += 2.0 * PI;
        }
        Self::from_radians(val)
    }
}

// TODO: Image with "envelope"
#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub struct Envelope {
    pub attack_length: u16,
    pub attack_level: u16,
    pub fade_length: u16,
    pub fade_level: u16,
}

/// Defines scheduling of the force-feedback effect
#[derive(Copy, Clone, PartialEq, Debug, Default)]
#[repr(C)]
pub struct Replay {
    pub length: u16,
    pub delay: u16,
}

/// Defines what triggers the force-feedback effect
#[derive(Copy, Clone, PartialEq, Debug, Default)]
#[repr(C)]
pub struct Trigger {
    pub button: Button,
    pub interval: u16,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Error {
    /// There is not enough space in device for new effect
    NotEnoughSpace,
    /// Force feedback is not supported by device
    FfNotSupported,
    /// Requested effect is not supported by device
    EffectNotSupported,
}

impl Error {
    pub fn to_str(self) -> &'static str {
        match self {
            Error::NotEnoughSpace => "not enough space for new effect",
            Error::FfNotSupported => "force feedback is not supported",
            Error::EffectNotSupported => "effect is not supported by device",
        }
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        self.to_str()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(self.to_str())
    }
}
