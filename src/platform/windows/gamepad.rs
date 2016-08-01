#![allow(unused_variables)]

use gamepad::{Event, Status};
use uuid::Uuid;

#[derive(Debug)]
pub struct Gilrs {
    pub gamepads: Vec<Gamepad>,
}

impl Gilrs {
    pub fn new() -> Self {
        Gilrs { gamepads: Vec::new() }
    }

    pub fn handle_hotplug(&mut self) -> Option<(Gamepad, Status)> {
        None
    }
}

#[derive(Debug)]
pub struct Gamepad {
    pub name: String,
    pub uuid: Uuid,
}

impl Gamepad {
    /// Returns gamepad that had never existed. All actions performed on returned object are no-op.
    pub fn none() -> Self {
        Gamepad {
            name: String::new(),
            uuid: Uuid::nil(),
        }
    }

    pub fn eq_disconnect(&self, other: &Self) -> bool {
        false
    }

    pub fn event(&mut self) -> Option<Event> {
        None
    }

    pub fn disconnect(&mut self) {}

    pub fn max_ff_effects(&self) -> usize {
        0
    }

    pub fn is_ff_supported(&self) -> bool {
        false
    }

    pub fn set_ff_gain(&mut self, gain: u16) {}
}

impl PartialEq for Gamepad {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
    }
}

pub mod native_ev_codes {
    pub const BTN_SOUTH: u16 = 0;
    pub const BTN_EAST: u16 = 1;
    pub const BTN_C: u16 = 2;
    pub const BTN_NORTH: u16 = 3;
    pub const BTN_WEST: u16 = 4;
    pub const BTN_Z: u16 = 5;
    pub const BTN_LT: u16 = 6;
    pub const BTN_RT: u16 = 7;
    pub const BTN_LT2: u16 = 8;
    pub const BTN_RT2: u16 = 9;
    pub const BTN_SELECT: u16 = 10;
    pub const BTN_START: u16 = 11;
    pub const BTN_MODE: u16 = 12;
    pub const BTN_LTHUMB: u16 = 13;
    pub const BTN_RTHUMB: u16 = 14;

    pub const BTN_DPAD_UP: u16 = 15;
    pub const BTN_DPAD_DOWN: u16 = 16;
    pub const BTN_DPAD_LEFT: u16 = 17;
    pub const BTN_DPAD_RIGHT: u16 = 18;

    pub const AXIS_LSTICKX: u16 = 0;
    pub const AXIS_LSTICKY: u16 = 1;
    pub const AXIS_LEFTZ: u16 = 2;
    pub const AXIS_RSTICKX: u16 = 3;
    pub const AXIS_RSTICKY: u16 = 4;
    pub const AXIS_RIGHTZ: u16 = 5;
    pub const AXIS_DPADX: u16 = 6;
    pub const AXIS_DPADY: u16 = 7;
    pub const AXIS_RT: u16 = 8;
    pub const AXIS_LT: u16 = 9;
    pub const AXIS_RT2: u16 = 10;
    pub const AXIS_LT2: u16 = 11;
}
