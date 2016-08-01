mod gamepad;
mod ff;

pub use self::gamepad::{Gilrs, Gamepad, native_ev_codes};
pub use self::ff::Effect;

pub const NAME: &'static str = "Unknown";
