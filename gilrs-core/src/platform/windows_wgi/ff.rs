// Copyright 2016-2018 Mateusz Sieczko and other GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
use std::time::Duration;
use windows::Gaming::Input::RawGameController;

#[derive(Debug)]
pub struct Device {
    raw_game_controller: RawGameController,
}

impl Device {
    pub(crate) fn new(raw_game_controller: RawGameController) -> Self {
        Device {
            raw_game_controller,
        }
    }

    pub fn set_ff_state(&mut self, strong: u16, weak: u16, _min_duration: Duration) {
        // todo!()
    }
}
