// Copyright 2016 GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
mod gamepad;
mod ff;

pub use self::ff::Device as FfDevice;
pub use self::gamepad::{native_ev_codes, Gamepad, Gilrs};

// Platform name used in SDL mappings format
pub const NAME: &'static str = "Unknown";
