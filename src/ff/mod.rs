// Copyright 2016 GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

pub(crate) mod server;
mod base_effect;
mod effect_source;
mod time;

pub(crate) use self::time::TICK_DURATION;
pub use self::time::{Ticks, Repeat};
pub use self::base_effect::{BaseEffect, BaseEffectType, Envelope, Replay};
pub use self::effect_source::{DistanceModel, DistanceModelError};

use std::{fmt, u32, f32};
use std::error::Error as StdError;
use std::sync::mpsc::{Sender, SendError};

use self::effect_source::{EffectSource};
use gamepad::Gilrs;
use ff::server::Message;
use utils;

use vec_map::VecMap;

pub struct Effect {
    id: usize,
    tx: Sender<Message>,
}

impl Clone for Effect {
    fn clone(&self) -> Self {
        let _ = self.tx.send(Message::HandleCloned { id: self.id });
        Effect {
            id: self.id,
            tx: self.tx.clone(),
        }
    }
}

impl Drop for Effect {
    fn drop(&mut self) {
        let _ = self.tx.send(Message::HandleDropped { id: self.id });
    }
}

impl Effect {
    pub fn play(&self) {
        let _ = self.tx.send(Message::Play { id: self.id });
    }

    pub fn set_gamepads(&self, ids: &[usize], gilrs: &Gilrs) -> Result<(), Error> {
        let mut gamepads = VecMap::new();

        for dev in ids.iter().cloned() {
            if !gilrs.connected_gamepad(dev).ok_or(Error::Disconnected(dev))?.is_ff_supported() {
                return Err(Error::FfNotSupported(dev));
            } else {
                gamepads.insert(dev, ());
            }
        }

        self.tx.send(Message::SetGamepads { id: self.id, gamepads })?;

        Ok(())
    }

    pub fn set_repeat(&self, repeat: Repeat) -> Result<(), Error> {
        self.tx.send(Message::SetRepeat {id: self.id, repeat })?;

        Ok(())
    }

    pub fn set_distance_model(&self, model: DistanceModel) -> Result<(), Error> {
        model.validate()?;
        self.tx.send(Message::SetDistanceModel { id: self.id, model })?;

        Ok(())
    }

    pub fn set_position<Vec3f: Into<[f32; 3]>>(&self, position: Vec3f) -> Result<(), Error> {
        let position = position.into();
        self.tx.send(Message::SetPosition  { id: self.id, position })?;

        Ok(())
    }

    pub fn set_gain(&self, gain: f32) -> Result<(), Error> {
        let gain = utils::clamp(gain, 0.0, f32::MAX);
        self.tx.send(Message::SetGain  { id: self.id, gain })?;

        Ok(())
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
            dist_model: DistanceModel::None,
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

    pub fn distance_model(&mut self, model: DistanceModel) -> &mut Self {
        self.dist_model = model;
        self
    }

    pub fn position<Vec3f: Into<[f32; 3]>>(&mut self, position: Vec3f) -> &mut Self {
        self.position = position.into();
        self
    }

    pub fn gain(&mut self, gain: f32) -> &mut Self {
        self.gain = utils::clamp(gain, 0.0, f32::MAX);
        self
    }

    pub fn finish(&mut self, gilrs: &mut Gilrs) -> Result<Effect, Error> {
        for (dev, _) in &self.devices {
            if !gilrs.connected_gamepad(dev).ok_or(Error::Disconnected(dev))?.is_ff_supported() {
                return Err(Error::FfNotSupported(dev));
            }
        }

        self.dist_model.validate()?;

        let effect = EffectSource::new(self.base_effects.clone(), self.devices.clone(),
                                       self.repeat, self.dist_model,
                                       self.position, self.gain);
        let id = gilrs.next_ff_id();
        let tx = gilrs.ff_sender();
        tx.send(Message::Create { id, effect: Box::new(effect) })?;
        Ok(Effect { id, tx: tx.clone() })
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Error {
    /// Force feedback is not supported by device with this ID
    FfNotSupported(usize),
    /// Device is not connected
    Disconnected(usize),
    /// Distance model is invalid.
    InvalidDistanceModel(DistanceModelError),
    /// The other end of channel was dropped.
    SendError,
    /// Unexpected error has occurred
    Other,
    #[doc(hidden)]
    __Nonexhaustive,
}

impl StdError for Error {
    fn description(&self) -> &str {
        match *self {
            Error::FfNotSupported(_) => "force feedback is not supported",
            Error::Disconnected(_) => "device is not connected",
            Error::InvalidDistanceModel(_) => "distance model is invalid",
            Error::SendError => "receiving end of a channel is disconnected",
            Error::Other => "unexpected error has occurred",
            Error::__Nonexhaustive => unreachable!(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(
            &match *self {
                Error::FfNotSupported(id) =>
                    format!("Force feedback is not supported by device with id {}.", id),
                Error::Disconnected(id) =>
                    format!("Device with id {} is not connected.", id),
                Error::InvalidDistanceModel(err)
                    => format!("Distance model is invalid: {}.", err.description()),
                Error::SendError => "Receiving end of a channel is disconnected.".to_owned(),
                Error::Other => "Unexpected error has occurred.".to_owned(),
                Error::__Nonexhaustive => unreachable!(),
        })
    }
}

impl<T> From<SendError<T>> for Error {
    fn from(_: SendError<T>) -> Self {
        Error::SendError
    }
}

impl From<DistanceModelError> for Error {
    fn from(f: DistanceModelError) -> Self {
        Error::InvalidDistanceModel(f)
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
