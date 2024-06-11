// Copyright 2016-2018 Mateusz Sieczko and other GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

//! Module which exports the platform-specific types.
//!
//! Each backend has to provide:
//!
//! * A `FfDevice` (a struct which handles force feedback)
//! * A `Gilrs` context
//! * A `Gamepad` struct
//! * A static `str` which specifies the name of the SDL input mapping
//! * A constant which define whether Y axis of sticks points upwards or downwards
//! * A module with the platform-specific constants for common gamepad buttons
//!   called `native_ev_codes`

#![allow(clippy::module_inception)]

pub use self::platform::*;

#[cfg(any(target_os = "linux"))]
#[path = "linux/mod.rs"]
mod platform;

#[cfg(target_os = "macos")]
#[path = "macos/mod.rs"]
mod platform;

#[cfg(all(not(feature = "xinput"), not(feature = "wgi")))]
compile_error!(
    "Windows needs one of the features `gilrs/xinput` or `gilrs/wgi` enabled. \nEither don't use \
     'default-features = false' or add one of the features back."
);

#[cfg(all(feature = "wgi", feature = "xinput"))]
compile_error!("features `gilrs/xinput` and `gilrs/wgi` are mutually exclusive");

#[cfg(all(target_os = "windows", feature = "xinput", not(feature = "wgi")))]
#[path = "windows_xinput/mod.rs"]
mod platform;

#[cfg(all(target_os = "windows", feature = "wgi"))]
#[path = "windows_wgi/mod.rs"]
mod platform;

#[cfg(target_arch = "wasm32")]
#[path = "wasm/mod.rs"]
mod platform;

#[cfg(all(
    not(any(target_os = "linux")),
    not(target_os = "macos"),
    not(target_os = "windows"),
    not(target_arch = "wasm32")
))]
#[path = "default/mod.rs"]
mod platform;
