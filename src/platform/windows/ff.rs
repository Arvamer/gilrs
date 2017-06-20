// Copyright 2016 GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use winapi::xinput::XINPUT_VIBRATION as XInputVibration;
use winapi::winerror::{ERROR_SUCCESS, ERROR_DEVICE_NOT_CONNECTED};
use xinput;

#[derive(Debug)]
pub struct Device {
    id: u32,
}

impl Device {
    pub fn new(id: u32) -> Self {
        Device { id }
    }

    pub(crate) fn set_ff_state(&mut self, strong: u16, weak: u16) {
        let mut effect = XInputVibration { wLeftMotorSpeed: strong, wRightMotorSpeed: weak };
        unsafe {
            let err = xinput::XInputSetState(self.id, &mut effect);
            match err {
                ERROR_SUCCESS => (),
                ERROR_DEVICE_NOT_CONNECTED => {
                    error!("Failed to change FF state – gamepad with id {} is no \
                                        longer connected.",
                           self.id);
                }
                _ => {
                    error!("Failed to change FF state – unknown error. ID = {}, \
                                        error code = {}.",
                           self.id,
                           err);
                }
            }
        }
    }
}
