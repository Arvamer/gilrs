#[cfg(target_os = "linux")]
extern crate libudev_sys;
#[cfg(target_os = "linux")]
extern crate libc;
#[cfg(target_os = "linux")]
extern crate ioctl;

extern crate vec_map;

mod gamepad;
mod platform;

pub mod constants;
pub mod ff;

pub use gamepad::{Gilrs, Gamepad, EventIterator, GamepadState, Status, Button, Axis};
