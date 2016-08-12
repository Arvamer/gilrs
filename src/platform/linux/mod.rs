mod gamepad;
mod udev;
mod ff;

pub use self::gamepad::{Gilrs, Gamepad, native_ev_codes, EventIterator};
pub use self::ff::Effect;

pub const NAME: &'static str = "Linux";
