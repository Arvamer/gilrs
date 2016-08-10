mod gamepad;
mod ff;

pub use self::gamepad::{Gilrs, Gamepad, EventIterator, native_ev_codes};
pub use self::ff::Effect;

pub const NAME: &'static str = "Windows";
