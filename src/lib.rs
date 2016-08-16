// Copyright 2016 GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
//! GilRs - Game Input Library for Rust
//! ===================================
//!
//! GilRs abstract platform specific APIs to provide unified interfaces for working with gamepads.
//! Additionally, library is trying to unify different devices, providing single controller layout.
//!
//! Example
//! -------
//!
//! ```
//! use gilrs::{Gilrs, Button};
//!
//! let mut gilrs = Gilrs::new();
//!
//! // Event loop
//! loop {
//!     for (id, event) in gilrs.pool_events() {
//!         println!("New event from {}: {:?}", id, event);
//!     }
//!
//!     if gilrs.gamepad(0).is_btn_pressed(Button::South) {
//!         println!("Name of gamepad 0: {}", gilrs.gamepad(0).name());
//!     }
//!     # break;
//! }
//! ```
//!
//! Supported features
//! ------------------
//!
//! |                  | Input | Hotplugging | Mappings | Force feedback |
//! |------------------|:-----:|:-----------:|:--------:|:--------------:|
//! | Linux            |   ✓   |      ✓      |     ✓    |        ✓       |
//! | Windows (XInput) |   ✓   |      ✓      |    n/a   |        ❌      |
//! | Windows (DInput) |   ❌  |      ❌     |     ❌   |        ❌      |
//! | OS X             |   ❌  |      ❌     |     ❌   |        ❌      |
//! | Android          |   ❌  |      ❌     |     ❌   |        ❌      |
//!
//! Controller layout
//! -----------------
//!
//! ![Controller layout](https://arvamer.gitlab.io/gilrs/img/controller.svg)
//! [original image by nicefrog](http://opengameart.org/content/generic-gamepad-template)
//!
//! Platform specific notes
//! ======================
//!
//! Linux
//! -----
//!
//! On Linux, GilRs read (and write, in case of force feedback) directly from appropriate
//! `/dev/input/event*` file. This mean that user have to have read and write access to this file.
//! On most distros it shouldn't be a problem, but if it is, you will have to create udev rule.

#[cfg(target_os = "linux")]
extern crate libudev_sys;
#[cfg(target_os = "linux")]
extern crate libc;
#[cfg(target_os = "linux")]
extern crate ioctl;

#[cfg(target_os = "windows")]
extern crate winapi;
#[cfg(target_os = "windows")]
extern crate xinput;

extern crate vec_map;
extern crate uuid;
#[macro_use]
extern crate log;

mod gamepad;
mod platform;
mod constants;
mod mapping;

pub mod ff;

pub use gamepad::{Gilrs, Gamepad, EventIterator, GamepadState, Status, Button, Axis, Event};

trait AsInner<T> {
    fn as_inner(&self) -> &T;
    fn as_inner_mut(&mut self) -> &mut T;
}
