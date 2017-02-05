// Copyright 2016 GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
#![allow(unused_variables)]

use super::gamepad::Gamepad;
use ff::{EffectData, Error};
use std::sync::mpsc::SyncSender;

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
}

