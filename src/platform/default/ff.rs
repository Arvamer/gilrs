// Copyright 2016 GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
#![allow(unused_variables)]

use super::gamepad::Gamepad;
use ff::{EffectData, Error};

#[derive(Debug)]
pub struct Effect {}

impl Effect {
    pub fn new(gamepad: &Gamepad, data: EffectData) -> Result<Self, Error> {
        Err(Error::FfNotSupported)
    }

    pub fn upload(&mut self, data: EffectData) -> Result<(), Error> {
        Err(Error::FfNotSupported)
    }

    pub fn play(&mut self, n: u16) -> Result<(), Error> {
        Err(Error::FfNotSupported)
    }

    pub fn stop(&mut self) -> Result<(), Error> {
        Err(Error::FfNotSupported)
    }
}
