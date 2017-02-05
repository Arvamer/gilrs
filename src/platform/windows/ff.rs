// Copyright 2016 GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use super::gamepad::Gamepad;
use ff::{EffectData, Error, EffectType};
use std::sync::mpsc::{SyncSender, TrySendError};
use std::time::{Instant, Duration};
use winapi::xinput::XINPUT_VIBRATION as XInputVibration;
use winapi::winerror::{ERROR_SUCCESS, ERROR_DEVICE_NOT_CONNECTED};
use xinput;

// Same as in Linux memless ff driver
pub const MAX_EFFECTS: usize = 16;

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

        self.tx.try_send(FfMessage {
            id: self.id,
            idx: self.idx,
            kind: FfMessageType::Create(data),
        })?;
        Ok(())
    }

    pub fn play(&mut self, n: u16) -> Result<(), Error> {
        self.tx.try_send(FfMessage {
            id: self.id,
            idx: self.idx,
            kind: FfMessageType::Play(n),
        })?;
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), Error> {
        self.tx.try_send(FfMessage {
            id: self.id,
            idx: self.idx,
            kind: FfMessageType::Stop,
        })?;
        Ok(())
    }
}

impl Drop for Effect {
    fn drop(&mut self) {
        let msg = FfMessage {
            id: self.id,
            idx: self.idx,
            kind: FfMessageType::Drop,
        };
        let r = self.tx.try_send(msg);
        match r {
            Err(TrySendError::Full(_)) => {
                warn!("Dropping {:?} will block thread.", self);
                let _ = self.tx.send(msg);
            }
            _ => (),
        }
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
struct EffectInternal {
    pub data: EffectData,
    pub repeat: u16,
    pub waiting: bool,
    pub time: Instant,
}

impl EffectInternal {
    pub fn play(&mut self, n: u16) {
        self.repeat = n.saturating_add(1);
        if self.data.replay.delay != 0 {
            self.waiting = true;
        }
    }

    pub fn stop(&mut self) {
        self.repeat = 0;
    }

    fn set_ff_state(id: u8, mut effect: XInputVibration) {
        unsafe {
            let err = xinput::XInputSetState(id as u32, &mut effect);
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


#[derive(Copy, Clone)]
pub struct Device {
    effects: [Option<EffectInternal>; MAX_EFFECTS],
    gain: f32,
    id: u8,
}

impl Device {
    pub fn new(id: u8) -> Self {
        Device {
            effects: [None; 16],
            gain: 1.0,
            id: id,
        }
    }

    pub fn play(&mut self, idx: u8, n: u16) {
        self.effects[idx as usize].map(|mut e| e.play(n));
    }

    pub fn stop(&mut self, idx: u8) {
        self.effects[idx as usize].map(|mut e| e.stop());
    }

    pub fn set_gain(&mut self, gain: f32) {
        self.gain = gain;
    }

    pub fn drop(&mut self, idx: u8) {
        self.effects[idx as usize] = None;
    }

    pub fn create(&mut self, idx: u8, effect: EffectData) {
        self.effects[idx as usize] = Some(effect.into());
    }

    pub fn combine_and_play(&mut self) {
        let mut strong = 0u16;
        let mut weak = 0u16;

        for effect in self.effects.iter_mut() {
            let effect = match effect.as_mut() {
                Some(e) => e,
                None => continue,
            };

            let data = &effect.data;

            let dur = ms(Instant::now().duration_since(effect.time));

            if effect.repeat == 0 || dur < data.replay.delay as u32 {
                // Nothing to play here
                continue;
            }

            let total_length = data.replay.length as u32 + data.replay.delay as u32;
            if dur > total_length {
                // Effect iteration ended
                effect.repeat -= 1;
                effect.time = Instant::now();

                if effect.repeat == 0 {
                    // End of effect
                    continue;
                }

                if dur as u32 - total_length < data.replay.delay as u32 {
                    continue;
                }
            }

            let (next_strong, next_weak) = match data.kind {
                EffectType::Rumble { strong, weak } => ((strong as f32 * self.gain) as u16, (weak as f32 * self.gain) as u16),
                _ => unreachable!(),
            };
            strong = strong.saturating_add(next_strong);
            weak = weak.saturating_add(next_weak);
        }

        let effect = XInputVibration {
            wLeftMotorSpeed: strong,
            wRightMotorSpeed: weak,
        };

        EffectInternal::set_ff_state(self.id, effect);
    }
}

fn ms(dur: Duration) -> u32 {
    dur.as_secs() as u32 + (dur.subsec_nanos() as f64 / 1_000_000.0) as u32
}
