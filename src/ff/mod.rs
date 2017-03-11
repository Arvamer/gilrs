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
//! use gilrs::Gilrs;
//! use gilrs::ff::{EffectData, EffectType, Waveform, Envelope};
//!
//! let mut gilrs = Gilrs::new();
//! let effect = EffectData {
//!     kind: EffectType::Periodic {
//!         wave: Waveform::Sine,
//!         period: 1000,
//!         magnitude: 30_000,
//!         offset: 0,
//!         phase: 0,
//!         envelope: Envelope {
//!             attack_length: 1000,
//!             attack_level: 0,
//!             fade_length: 1000,
//!             fade_level: 0,
//!         }
//!     },
//!     .. Default::default()
//! };
//!
//! let effect_idx = gilrs[0].add_ff_effect(effect).unwrap();
//! gilrs[0].ff_effect(effect_idx).unwrap().play(1).unwrap();
//! ```

pub(crate) mod server;

use std::{fmt, u16, u32};
use std::error::Error as StdError;
use std::sync::mpsc::{Sender, TrySendError};
use std::ops::{Mul, AddAssign, Add, Rem, Sub, SubAssign};

use gamepad::Gilrs;
use ff::server::Message;
use utils;

use vec_map::VecMap;

pub(crate) const TICK_DURATION: u32 = 50;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Ticks(u32);

impl Ticks {
    pub fn from_ms(dur: u32) -> Self {
        Ticks(utils::ceil_div(dur, TICK_DURATION))
    }

    fn inc(&mut self) {
        self.0 += 1
    }

    fn checked_sub(self, rhs: Ticks) -> Option<Ticks> {
        self.0.checked_sub(rhs.0).map(|t| Ticks(t))
    }
}

impl Add for Ticks {
    type Output = Ticks;

    fn add(self, rhs: Ticks) -> Self::Output {
        Ticks(self.0 + rhs.0)
    }
}

impl AddAssign for Ticks {
    fn add_assign(&mut self, rhs: Ticks) {
        self.0 += rhs.0
    }
}

impl Sub for Ticks {
    type Output = Ticks;

    fn sub(self, rhs: Ticks) -> Self::Output {
        Ticks(self.0 - rhs.0)
    }
}

impl SubAssign for Ticks {
    fn sub_assign(&mut self, rhs: Ticks) {
        self.0 -= rhs.0
    }
}

impl Rem for Ticks {
    type Output = Ticks;

    fn rem(self, rhs: Ticks) -> Self::Output {
        Ticks(self.0 % rhs.0)
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum BaseEffectType {
    Weak { magnitude: u16 },
    Strong { magnitude: u16 },
    #[doc(hidden)]
    __Nonexhaustive,
}

impl BaseEffectType {
    fn magnitude(&self) -> u16 {
        match *self {
            BaseEffectType::Weak { magnitude } => magnitude,
            BaseEffectType::Strong { magnitude } => magnitude,
            BaseEffectType::__Nonexhaustive => unreachable!(),
        }
    }

    fn weaken(self, att: f32) -> Self {
        let mg = (self.magnitude() as f32 * att) as u16;
        match self {
            BaseEffectType::Weak { .. } => BaseEffectType::Weak { magnitude: mg },
            BaseEffectType::Strong { .. } => BaseEffectType::Weak { magnitude: mg },
            BaseEffectType::__Nonexhaustive => unreachable!(),
        }
    }
}

impl Default for BaseEffectType {
    fn default() -> Self {
        BaseEffectType::Weak { magnitude: 0 }
    }
}

#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub struct BaseEffect {
    pub kind: BaseEffectType,
    pub scheduling: Replay,
    // TODO: maybe allow other f(t)?
    pub envelope: Envelope,
}

impl BaseEffect {
    /// Returns `Weak` or `Strong` after applying envelope.
    fn magnitude_at(&self, ticks: Ticks) -> BaseEffectType {
        if let Some(wrapped) = self.scheduling.wrap(ticks) {
            let att = self.scheduling.at(wrapped) * self.envelope.at(wrapped, self.scheduling.play_for);
            self.kind.weaken(att)
        } else {
            self.kind.weaken(0.0)
        }
    }
}

// TODO: Image with "envelope"
#[derive(Copy, Clone, PartialEq, Debug, Default)]
/// Envelope shaped gain(time) function.
pub struct Envelope {
    pub attack_length: Ticks,
    pub attack_level: f32,
    pub fade_length: Ticks,
    pub fade_level: f32,
}

impl Envelope {
    fn at(&self, ticks: Ticks, dur: Ticks) -> f32 {
        debug_assert!(self.fade_length < dur);
        debug_assert!(self.attack_length + self.fade_length < dur);

        if ticks < self.attack_length {
            self.attack_level + ticks.0 as f32 * (1.0 - self.attack_level) / self.attack_length.0 as f32
        } else if ticks + self.fade_length > dur {
            1.0 + (ticks + self.fade_length - dur).0 as f32 * (self.fade_level - 1.0) / self.fade_length.0 as f32
        } else {
            1.0
        }
    }
}

/// Defines scheduling of the force feedback effect
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Replay {
    pub after: Ticks,
    pub play_for: Ticks,
    pub with_delay: Ticks,
}

impl Replay {
    fn at(&self, ticks: Ticks) -> f32 {
        match ticks.checked_sub(self.after) {
            Some(ticks) => {
                if ticks.0 >= self.play_for.0 {
                    0.0
                } else {
                    1.0
                }
            }
            None => 0.0,
        }
    }

    /// Returns duration of effect calculated as `play_for + with_delay`.
    pub fn dur(&self) -> Ticks {
        self.play_for + self.with_delay
    }

    /// Returns `None` if effect hasn't started or wrapped value
    fn wrap(&self, ticks: Ticks) -> Option<Ticks> {
        ticks.checked_sub(self.after).map(|t| t % self.dur())
    }
}

impl Default for Replay {
    fn default() -> Self {
        Replay {
            after: Ticks(0),
            play_for: Ticks(1),
            with_delay: Ticks(0),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Repeat {
    Infinitely,
    For(Ticks),
}

impl Default for Repeat {
    fn default() -> Self {
        Repeat::Infinitely
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum DistanceModel {
    Constant,
    Linear { ref_distance: f32 },
}

impl DistanceModel {
    fn attenuation(self, distance: f32) -> f32 {
        match self {
            DistanceModel::Linear { .. } => unimplemented!(),
            DistanceModel::Constant => 1.0,
        }
    }
}

impl Default for DistanceModel {
    fn default() -> Self {
        DistanceModel::Constant
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
enum EffectState {
    Playing { since: Ticks },
    Stopped,
}

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct EffectSource {
    base_effects: Vec<BaseEffect>,
    devices: VecMap<()>,
    repeat: Repeat,
    dist_model: DistanceModel,
    position: [f32; 3],
    gain: f32,
    state: EffectState,
}

impl EffectSource {
    fn combine_base_effects(&self, ticks: Ticks, actor_pos: [f32; 3]) -> Magnitude {
        let ticks = match self.state {
            EffectState::Playing { since } =>{
                debug_assert!(ticks >= since);
                ticks - since
            },
            EffectState::Stopped => return Magnitude::zero(),
        };

        match self.repeat {
            Repeat::For(max_dur) if max_dur > ticks => {
                // TODO: Maybe change to new state, "Ended"?
                // self.state = EffectState::Stopped;
                return Magnitude::zero();
            }
            _ => ()
        }

        let attenuation = self.dist_model.attenuation(self.position.distance(actor_pos)) * self.gain;
        if attenuation < 0.05 {
            return Magnitude::zero()
        }

        let mut final_magnitude = Magnitude::zero();
        for effect in &self.base_effects {
            match effect.magnitude_at(ticks) {
                BaseEffectType::Strong { magnitude } => final_magnitude.strong = final_magnitude.strong.saturating_add(magnitude),
                BaseEffectType::Weak { magnitude } => final_magnitude.weak = final_magnitude.weak.saturating_add(magnitude),
                BaseEffectType::__Nonexhaustive => (),
            };
        }
        final_magnitude * attenuation
    }
}

/// (strong, weak) pair.
#[derive(Copy, Clone, Debug)]
pub(crate) struct Magnitude {
    pub strong: u16,
    pub weak: u16,
}

impl Magnitude {
    pub fn zero() -> Self {
        Magnitude { strong: 0, weak: 0 }
    }
}

impl Mul<f32> for Magnitude {
    type Output = Magnitude;

    fn mul(self, rhs: f32) -> Self::Output {
        debug_assert!(rhs >= 0.0);
        let strong = self.strong as f32 * rhs;
        let strong = if strong > u16::MAX as f32 { u16::MAX } else { strong as u16 };
        let weak = self.weak as f32 * rhs;
        let weak = if weak > u16::MAX as f32 { u16::MAX } else { weak as u16 };
        Magnitude { strong: strong, weak: weak }
    }
}

impl AddAssign for Magnitude {
    fn add_assign(&mut self, rhs: Magnitude) {
        self.strong = self.strong.saturating_add(rhs.strong);
        self.weak = self.weak.saturating_add(rhs.weak);
    }
}

pub struct Effect {
    id: usize,
    tx: Sender<Message>,
}

impl Effect {
    pub fn play(&self) {
        let _ = self.tx.send(Message::Play { id: self.id });
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct EffectBuilder {
    base_effects: Vec<BaseEffect>,
    devices: VecMap<()>,
    repeat: Repeat,
    dist_model: DistanceModel,
    position: [f32; 3],
    gain: f32,
}

impl EffectBuilder {
    pub fn new() -> Self {
        EffectBuilder {
            base_effects: Vec::new(),
            devices: VecMap::new(),
            repeat: Repeat::Infinitely,
            dist_model: DistanceModel::Constant,
            position: [0.0, 0.0, 0.0],
            gain: 1.0,
        }
    }

    pub fn add_effect(&mut self, effect: BaseEffect) -> &mut Self {
        self.base_effects.push(effect);
        self
    }

    pub fn gamepads(&mut self, ids: &[usize]) -> &mut Self {
        for dev in ids {
            self.devices.insert(*dev, ());
        }
        self
    }

    pub fn repeat(&mut self, repeat: Repeat) -> &mut Self {
        self.repeat = repeat;
        self
    }

    pub fn dist_model(&mut self, model: DistanceModel) -> &mut Self {
        self.dist_model = model;
        self
    }

    pub fn position<Vec3f: Into<[f32; 3]>>(&mut self, position: Vec3f) -> &mut Self {
        self.position = position.into();
        self
    }

    pub fn gain(&mut self, gain: f32) -> &mut Self {
        assert!(gain >= 0.0);
        self.gain = gain;
        self
    }

    pub fn finish(&mut self, gilrs: &mut Gilrs) -> Result<Effect, Error> {
        for (dev, _) in &self.devices {
            if !gilrs.connected_gamepad(dev).ok_or(Error::Disconnected)?.is_ff_supported() {
                return Err(Error::FfNotSupported);
            }
        }

        let effect =  EffectSource {
            base_effects: self.base_effects.clone(),
            devices: self.devices.clone(),
            repeat: self.repeat,
            dist_model: self.dist_model,
            position: self.position,
            gain: self.gain,
            state: EffectState::Stopped,
        };

        let id = gilrs.next_ff_id();
        let tx = gilrs.ff_sender();
        tx.send(Message::Update { id, effect }).or(Err(Error::Other))?;
        Ok(Effect { id, tx: tx.clone() })
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Error {
    /// There is not enough space in device for new effect
    NotEnoughSpace,
    /// Force feedback is not supported by device
    FfNotSupported,
    /// Requested effect or action is not supported by device/driver
    NotSupported,
    /// Can not play effect
    FailedToPlay,
    /// Device is not connected
    Disconnected,
    /// Effect with requested ID doesn't exist
    InvalidId,
    /// Sending force feedback command would block current thread. This can happen on Windows with
    /// most force feedback functions.
    WouldBlock,
    /// Unexpected error has occurred
    Other,
    #[doc(hidden)]
    __Nonexhaustive,
}

impl Error {
    pub fn to_str(self) -> &'static str {
        match self {
            Error::NotEnoughSpace => "not enough space for new effect",
            Error::FfNotSupported => "force feedback is not supported",
            Error::NotSupported => "effect or action is not supported by device",
            Error::FailedToPlay => "can't play effect",
            Error::Disconnected => "device is not connected",
            Error::InvalidId => "effect with requested ID doesn't exist",
            Error::WouldBlock => "this thread would be blocked by last ff operation",
            Error::Other => "unexpected error has occurred",
            Error::__Nonexhaustive => unreachable!(),
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

impl<T> From<TrySendError<T>> for Error {
    fn from(f: TrySendError<T>) -> Self {
        match f {
            TrySendError::Full(_) => Error::WouldBlock,
            _=> Error::Other,
        }
    }
}

trait SliceVecExt {
    type Base;

    fn distance(self, from: Self) -> Self::Base;
}

impl  SliceVecExt for [f32; 3] {
    type Base = f32;

    fn distance(self, from: Self) -> f32 {
        ((from[0] - self[0]).powi(2) + (from[1] - self[1]).powi(2) + (from[2] - self[2]).powi(2)).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::u32;

    #[test]
    fn envelope() {
        let env = Envelope {
            attack_length: Ticks(10),
            attack_level: 0.2,
            fade_length: Ticks(10),
            fade_level: 0.2,
        };
        let dur = Ticks(40);

        assert_eq!(env.at(Ticks(0), dur), 0.2);
        assert_eq!(env.at(Ticks(5), dur), 0.6);
        assert_eq!(env.at(Ticks(10), dur), 1.0);
        assert_eq!(env.at(Ticks(20), dur), 1.0);
        assert_eq!(env.at(Ticks(30), dur), 1.0);
        assert_eq!(env.at(Ticks(35), dur), 0.6);
        assert_eq!(env.at(Ticks(40), dur), 0.19999999);
    }

    #[test]
    fn envelope_default() {
        let env = Envelope::default();
        let dur = Ticks(40);

        assert_eq!(env.at(Ticks(0), dur), 1.0);
        assert_eq!(env.at(Ticks(20), dur), 1.0);
        assert_eq!(env.at(Ticks(40), dur), 1.0);
    }

    #[test]
    fn replay() {
        let replay = Replay {
            after: Ticks(10),
            play_for: Ticks(50),
            with_delay: Ticks(20),
        };

        assert_eq!(replay.at(Ticks(0)), 0.0);
        assert_eq!(replay.at(Ticks(9)), 0.0);
        assert_eq!(replay.at(Ticks(10)), 1.0);
        assert_eq!(replay.at(Ticks(30)), 1.0);
        assert_eq!(replay.at(Ticks(59)), 1.0);
        assert_eq!(replay.at(Ticks(60)), 0.0);
        assert_eq!(replay.at(Ticks(70)), 0.0);
        assert_eq!(replay.at(Ticks(79)), 0.0);
    }
}
