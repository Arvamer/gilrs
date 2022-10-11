// Copyright 2016-2018 Mateusz Sieczko and other GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
use std::time::Duration;
use windows::Gaming::Input::Gamepad as WgiGamepad;
use windows::Gaming::Input::GamepadVibration;

#[derive(Debug)]
pub struct Device {
    id: u32,
    wgi_gamepad: Option<WgiGamepad>,
}

impl Device {
    pub(crate) fn new(id: u32, wgi_gamepad: Option<WgiGamepad>) -> Self {
        Device { id, wgi_gamepad }
    }

    pub fn set_ff_state(&mut self, strong: u16, weak: u16, _min_duration: Duration) {
        if let Some(wgi_gamepad) = &self.wgi_gamepad {
            if let Err(err) = wgi_gamepad.SetVibration(GamepadVibration {
                LeftMotor: (strong as f64) / (u16::MAX as f64),
                RightMotor: (weak as f64) / (u16::MAX as f64),
                LeftTrigger: 0.0,
                RightTrigger: 0.0,
            }) {
                error!(
                    "Failed to change FF state – unknown error. ID = {}, error = {:?}.",
                    self.id, err
                );
            }
        }
    }
}
