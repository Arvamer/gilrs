// Copyright 2016-2018 Mateusz Sieczko and other GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use std::time::Duration;
use winapi::shared::winerror::{ERROR_DEVICE_NOT_CONNECTED, ERROR_SUCCESS};
use winapi::um::xinput::{self, XINPUT_VIBRATION as XInputVibration};

#[derive(Debug)]
pub struct Device {
    id: u32,
}

impl Device {
    pub(crate) fn new(id: u32) -> Self {
        Device { id }
    }

    pub fn set_ff_state(&mut self, strong: u16, weak: u16, _min_duration: Duration) {
        let mut effect = XInputVibration {
            wLeftMotorSpeed: strong,
            wRightMotorSpeed: weak,
        };
        unsafe {
            let err = xinput::XInputSetState(self.id, &mut effect);
            match err {
                ERROR_SUCCESS => (),
                ERROR_DEVICE_NOT_CONNECTED => {
                    error!(
                        "Failed to change FF state – gamepad with id {} is no longer connected.",
                        self.id
                    );
                }
                _ => {
                    error!(
                        "Failed to change FF state – unknown error. ID = {}, error code = {}.",
                        self.id, err
                    );
                }
            }
        }
    }
}
