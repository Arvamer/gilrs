use platform;

#[derive(Debug)]
pub struct Gilrs {
    gilrs: platform::Gilrs,
}

impl Gilrs {
    pub fn new() -> Self {
        Gilrs { gilrs: platform::Gilrs::new() }
    }

    pub fn pool_events(&mut self) -> EventIterator {
        EventIterator(self.gilrs.pool_events())
    }
}

#[derive(Debug)]
pub struct Gamepad {
    gamepad: platform::Gamepad,
}

pub struct EventIterator<'a>(platform::EventIterator<'a>);

impl<'a> Iterator for EventIterator<'a> {
    type Item = Event;

    fn next(&mut self) -> Option<Event> {
        self.0.next()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    ButtonPressed(Button),
    ButtonReleased(Button),
    AxisChanged(Axis, f32),
}

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Button {
    // Action Pad
    South = BTN_SOUTH,
    East = BTN_EAST,
    North = BTN_NORTH,
    West = BTN_WEST,
    C = BTN_C,
    Z = BTN_Z,
    // Triggers
    LeftTrigger = BTN_TL,
    LeftTrigger2 = BTN_TL2,
    RightTrigger = BTN_TR,
    RightTrigger2 = BTN_TR2,
    // Menu Pad
    Select = BTN_SELECT,
    Start = BTN_START,
    Mode = BTN_MODE,
    // Sticks
    LeftThumb = BTN_THUMBL,
    RightThumb = BTN_THUMBR,
    // D-Pad
    DPadUp = BTN_DPAD_UP,
    DPadDown = BTN_DPAD_DOWN,
    DPadLeft = BTN_DPAD_LEFT,
    DPadRight = BTN_DPAD_RIGHT,
}

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Axis {
    LeftStickX = ABS_X,
    LeftStickY = ABS_Y,
    RightStickX = ABS_RX,
    RightStickY = ABS_RY,
    LeftTrigger = ABS_HAT1Y,
    LeftTrigger2 = ABS_HAT2Y,
    RightTrigger = ABS_HAT1X,
    RightTrigger2 = ABS_HAT2X,
}

// Move this to platform::linux
const BTN_SOUTH: u16 = 0x130;
const BTN_EAST: u16 = 0x131;
const BTN_C: u16 = 0x132;
const BTN_NORTH: u16 = 0x133;
const BTN_WEST: u16 = 0x134;
const BTN_Z: u16 = 0x135;
const BTN_TL: u16 = 0x136;
const BTN_TR: u16 = 0x137;
const BTN_TL2: u16 = 0x138;
const BTN_TR2: u16 = 0x139;
const BTN_SELECT: u16 = 0x13a;
const BTN_START: u16 = 0x13b;
const BTN_MODE: u16 = 0x13c;
const BTN_THUMBL: u16 = 0x13d;
const BTN_THUMBR: u16 = 0x13e;

const BTN_DPAD_UP: u16 = 0x220;
const BTN_DPAD_DOWN: u16 = 0x221;
const BTN_DPAD_LEFT: u16 = 0x222;
const BTN_DPAD_RIGHT: u16 = 0x223;

const ABS_X: u16 = 0x00;
const ABS_Y: u16 = 0x01;
const ABS_RX: u16 = 0x03;
const ABS_RY: u16 = 0x04;
const ABS_HAT1X: u16 = 0x12;
const ABS_HAT1Y: u16 = 0x13;
const ABS_HAT2X: u16 = 0x14;
const ABS_HAT2Y: u16 = 0x15;
