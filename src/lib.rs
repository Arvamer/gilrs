// Copyright 2017 Mateusz Sieczko and other GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

//! GilRs - Game Input Library for Rust
//! ===================================
//!
//! GilRs abstract platform specific APIs to provide unified interfaces for working with gamepads.
//!
//! Main features:
//!
//! - Unified gamepad layout—buttons and axes are represented by familiar names
//! - Support for SDL2 mappings including `SDL_GAMECONTROLLERCONFIG` environment
//!   variable which Steam uses
//! - Hotplugging—GilRs will try to assign new IDs for new gamepads and reuse same
//!   ID for gamepads which reconnected
//! - Force feedback (rumble)
//! - Power information (is gamepad wired, current battery status)
//!
//! Example
//! -------
//!
//! ```
//! use gilrs::{Gilrs, Button, Event};
//!
//! let mut gilrs = Gilrs::new();
//!
//! // Iterate over all connected gamepads
//! for (_id, gamepad) in gilrs.gamepads() {
//!     println!("{} is {:?}", gamepad.name(), gamepad.power_info());
//! }
//!
//! loop {
//!     // Examine new events
//!     while let Some(Event { id, event, time }) = gilrs.next_event() {
//!         println!("{:?} New event from {}: {:?}", time, id, event);
//!     }
//!
//!     // You can also use cached gamepad state
//!     if gilrs[0].is_pressed(Button::South) {
//!         println!("Button South is pressed (XBox - A, PS - X)");
//!     }
//!     # break;
//! }
//! ```
//!
//! Supported features
//! ------------------
//!
//! |                  | Input | Hotplugging | Force feedback |
//! |------------------|:-----:|:-----------:|:--------------:|
//! | Linux            |   ✓   |      ✓      |        ✓       |
//! | Windows (XInput) |   ✓   |      ✓      |        ✓       |
//! | OS X             |   ✕   |      ✕      |        ✕       |
//! | Emscripten       |   ✕   |      ✕      |       n/a      |
//! | Android          |   ✕   |      ✕      |        ✕       |
//!
//!
//! Controller layout
//! -----------------
//!
//! ![Controller layout](https://arvamer.gitlab.io/gilrs/img/controller.svg)
//! [original image by nicefrog](http://opengameart.org/content/generic-gamepad-template)
//!
//! Mappings
//! --------
//!
//! GilRs use SDL-compatible controller mappings to fix on Linux legacy drivers that doesn't follow
//! [Linux Gamepad API](https://www.kernel.org/doc/Documentation/input/gamepad.txt) and to provide
//! unified button layout for platforms that doesn't make any guarantees about it. The main source
//! is [SDL_GameControllerDB](https://github.com/gabomdq/SDL_GameControllerDB), but library also
//! support loading mappings from environment variable `SDL_GAMECONTROLLERCONFIG` (which Steam
//! use).
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
//!
//! To build GilRs, you will need pkg-config and libudev .pc file. On some
//! distributions this file is packaged in separate archive (for example `libudev-dev` in Debian).

#[cfg(target_os = "linux")]
extern crate libc;
#[cfg(target_os = "linux")]
extern crate libudev_sys;
#[cfg(target_os = "linux")]
#[macro_use]
extern crate nix;

#[cfg(target_os = "windows")]
extern crate winapi;
#[cfg(target_os = "windows")]
extern crate xinput;

#[macro_use]
extern crate log;
extern crate uuid;
extern crate vec_map;

mod gamepad;
mod platform;
mod constants;
mod mapping;
mod utils;

pub mod ff;
pub mod ev;

pub use ev::filter::Filter;
pub use gamepad::{Axis, Button, ConnectedGamepadsIterator, ConnectedGamepadsMutIterator, Event,
                  EventType, Gamepad, Gilrs, GilrsBuilder, MappingSource, NativeEvCode, PowerInfo,
                  Status};
pub use mapping::{MappingData as Mapping, MappingError};

trait AsInner<T> {
    fn as_inner(&self) -> &T;
    fn as_inner_mut(&mut self) -> &mut T;
}
