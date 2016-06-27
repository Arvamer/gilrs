#[cfg(target_os = "linux")]
extern crate libudev_sys;
#[cfg(target_os = "linux")]
extern crate libc;
#[cfg(target_os = "linux")]
extern crate ioctl;

extern crate vec_map;

mod gamepad;
mod platform;

pub use gamepad::{Gilrs, Gamepad, EventIterator};
