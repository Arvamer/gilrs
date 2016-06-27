extern crate libudev_sys;
extern crate libc;
extern crate ioctl;

pub mod udev;
mod gamepad;

pub use gamepad::{Gilrs, Gamepad};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
