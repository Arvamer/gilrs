// Copyright 2016 GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
#![allow(unused_variables)]

use super::gamepad::Gamepad;
use ff::{EffectData, Error, EffectType};
use std::sync::mpsc::SyncSender;
use std::time::Instant;
use winapi::xinput::XINPUT_VIBRATION as XInputVibration;
use winapi::winerror::{ERROR_SUCCESS, ERROR_DEVICE_NOT_CONNECTED};
use xinput;

#[derive(Debug)]
pub struct Effect {
    /// ID of gamepad
    id: u8,
    /// Index of force feedback effect
    idx: u8,
    tx: SyncSender<FfMessage>,
}

impl Effect {
    pub fn new(gamepad: &Gamepad, data: EffectData) -> Result<Self, Error> {
        let idx = gamepad.get_free_ff_idx().ok_or(Error::NotEnoughSpace)?;
        let mut effect = Effect {
            id: gamepad.id(),
            idx: idx,
            tx: gamepad.ff_sender().clone(),
        };
        effect.upload(data)?;
        Ok(effect)
    }

    pub fn upload(&mut self, data: EffectData) -> Result<(), Error> {
        if !data.kind.is_rumble() {
            return Err(Error::NotSupported);
        }

        let _ = self.tx.send(FfMessage {
            id: self.id,
            idx: self.idx,
            kind: FfMessageType::Create(data),
        });
        Ok(())
    }

    pub fn play(&mut self, n: u16) -> Result<(), Error> {
        let _ = self.tx.send(FfMessage {
            id: self.id,
            idx: self.idx,
            kind: FfMessageType::Play(n),
        });
        Ok(())
    }

    pub fn stop(&mut self) {
        let _ = self.tx.send(FfMessage {
            id: self.id,
            idx: self.idx,
            kind: FfMessageType::Stop,
        });
    }
}

impl Drop for Effect {
    fn drop(&mut self) {
        self.stop();
        let _ = self.tx.send(FfMessage {
            id: self.id,
            idx: self.idx,
            kind: FfMessageType::Drop,
        });
    }
}

#[derive(Copy, Clone, Debug)]
pub struct FfMessage {
    /// ID of gamepad
    pub id: u8,
    /// Index of force feedback effect
    pub idx: u8,
    pub kind: FfMessageType,
}

#[derive(Copy, Clone, Debug)]
pub enum FfMessageType {
    Create(EffectData),
    Play(u16),
    Stop,
    Drop,
    ChangeGain(f32),
}

#[derive(Copy, Clone)]
pub struct EffectInternal {
    pub data: EffectData,
    pub repeat: u16,
    pub waiting: bool,
    pub time: Instant,
}

impl EffectInternal {
    pub fn play(&mut self, n: u16, id: u8, gain: f32) {
        self.repeat = n.saturating_add(1);
        if self.data.replay.delay != 0 {
            self.waiting = true;
        } else {
            self.play_effect(id, gain);
        }
    }

    pub fn stop(&mut self) {
        self.repeat = 0;
    }

    pub fn play_effect(&self, id: u8, gain: f32) {
        let (left, right) = match self.data.kind {
            EffectType::Rumble { strong, weak } => (strong as f32 * gain, weak as f32 * gain),
            _ => unreachable!(),
        };

        let mut effect = XInputVibration {
            wLeftMotorSpeed: left as u16,
            wRightMotorSpeed: right as u16,
        };

        Self::set_ff_state(id, &mut effect);
    }

    pub fn stop_effect(&self, id: u8) {
        let mut effect = XInputVibration {
            wLeftMotorSpeed: 0,
            wRightMotorSpeed: 0,
        };

        Self::set_ff_state(id, &mut effect);
    }

    fn set_ff_state(id: u8, effect: &mut XInputVibration) {
        unsafe {
            let err = xinput::XInputSetState(id as u32, effect);
            match err {
                ERROR_SUCCESS => (),
                ERROR_DEVICE_NOT_CONNECTED => {
                    error!("Failed to change FF state – gamepad with id {} is no \
                                        longer connected.",
                           id);
                }
                _ => {
                    error!("Failed to change FF state – unknown error. ID = {}, \
                                        error code = {}.",
                           id,
                           err);
                }
            }
        }
    }
}

impl From<EffectData> for EffectInternal {
    fn from(f: EffectData) -> Self {
        EffectInternal {
            data: f,
            repeat: 0,
            waiting: false,
            time: Instant::now(),
        }
    }
}
