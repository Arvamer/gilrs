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
