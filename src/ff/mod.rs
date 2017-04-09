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
pub use self::effect_source::DistanceModel;

use std::{fmt, u32};
use std::error::Error as StdError;
use std::sync::mpsc::{Sender, TrySendError};

use self::effect_source::{EffectSource};
use gamepad::Gilrs;
use ff::server::Message;

use vec_map::VecMap;

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

        let effect = EffectSource::new(self.base_effects.clone(), self.devices.clone(),
                                       self.repeat, self.dist_model,
                                       self.position, self.gain);
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
