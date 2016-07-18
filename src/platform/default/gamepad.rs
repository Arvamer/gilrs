#![allow(unused_variables)]

use gamepad::{Event, Status};

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
}

impl Gamepad {
    /// Returns gamepad that had never existed. All actions performed on returned object are no-op.
    pub fn none() -> Self {
        Gamepad {
            name: String::new(),
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
        false
    }
}
