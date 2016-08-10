mod gamepad;
mod ff;

pub use self::gamepad::{Gilrs, Gamepad, EventIterator, native_ev_codes};
pub use self::ff::Effect;

// Platform name used in SDL mappings format
pub const NAME: &'static str = "Unknown";
