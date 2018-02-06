// Copyright 2017 Mateusz Sieczko and other GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
// Copyright 2016 GilRs Developers
mod gamepad;
mod udev;
mod ff;
mod ioctl;

pub use self::ff::Device as FfDevice;
pub use self::gamepad::{native_ev_codes, EvCode, Gamepad, Gilrs};

pub const NAME: &'static str = "Linux";
pub const IS_Y_AXIS_REVERSED: bool = true;
