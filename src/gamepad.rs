use platform;
use std::mem;
use constants::*;
use ff::EffectData;
use uuid::Uuid;
use AsInner;

#[derive(Debug)]
pub struct Gilrs {
    inner: platform::Gilrs,
}

impl Gilrs {
    pub fn new() -> Self {
        Gilrs { inner: platform::Gilrs::new() }
    }

    pub fn pool_events(&mut self) -> EventIterator {
        EventIterator { inner: self.inner.pool_events() }
    }

    pub fn gamepad(&self, id: usize) -> &Gamepad {
        self.inner.gamepad(id)
    }

    pub fn gamepad_mut(&mut self, id: usize) -> &mut Gamepad {
        self.inner.gamepad_mut(id)
    }
}

#[derive(Debug)]
pub struct Gamepad {
    inner: platform::Gamepad,
    state: GamepadState,
    status: Status,
    ff_effects: Vec<Option<Effect>>,
}

impl Gamepad {
    fn new(gamepad: platform::Gamepad, status: Status) -> Self {
        let max_effects = gamepad.max_ff_effects();
        Gamepad {
            inner: gamepad,
            state: GamepadState::new(),
            status: status,
            ff_effects: (0..max_effects).map(|_| None).collect(),
        }
    }

    pub fn name(&self) -> &String {
        self.inner.name()
    }

    pub fn uuid(&self) -> Uuid {
        self.inner.uuid()
    }

    pub fn state(&self) -> &GamepadState {
        &self.state
    }

    pub fn status(&self) -> Status {
        self.status
    }

    pub fn is_pressed(&self, btn: Button) -> bool {
        let state = &self.state;
        match btn {
            Button::South => state.btn_south,
            Button::East => state.btn_east,
            Button::North => state.btn_north,
            Button::West => state.btn_west,
            Button::C => state.btn_c,
            Button::Z => state.btn_z,

            Button::LeftTrigger => state.left_trigger != 0.0,
            Button::LeftTrigger2 => state.left_trigger2 != 0.0,
            Button::RightTrigger => state.right_trigger != 0.0,
            Button::RightTrigger2 => state.right_trigger2 != 0.0,

            Button::Select => state.btn_select,
            Button::Start => state.btn_start,
            Button::Mode => state.btn_mode,

            Button::LeftThumb => state.btn_left_thumb,
            Button::RightThumb => state.btn_right_thumb,

            Button::DPadUp => state.btn_dpad_up,
            Button::DPadDown => state.btn_dpad_down,
            Button::DPadRight => state.btn_dpad_right,
            Button::DPadLeft => state.btn_dpad_left,

            Button::Unknow => false,
        }
    }

    pub fn axis_val(&self, axis: Axis) -> f32 {
        let state = &self.state;
        match axis {
            Axis::LeftStickX => state.left_stick.0,
            Axis::LeftStickY => state.left_stick.1,
            Axis::LeftZ => state.z.0,
            Axis::RightStickX => state.right_stick.0,
            Axis::RightStickY => state.right_stick.1,
            Axis::RightZ => state.z.1,
            Axis::LeftTrigger => state.left_trigger,
            Axis::LeftTrigger2 => state.left_trigger2,
            Axis::RightTrigger => state.right_trigger,
            Axis::RightTrigger2 => state.right_trigger2,
        }
    }

    pub fn add_ff_effect(&mut self, data: EffectData) -> Option<usize> {
        self.ff_effects.iter().position(|effect| effect.is_none()).and_then(|pos| {
            Effect::new(self, data).map(|effect| {
                unsafe {
                    *self.ff_effects.get_unchecked_mut(pos) = Some(effect);
                }
                pos
            })
        })
    }

    pub fn drop_ff_effect(&mut self, idx: usize) {
        self.ff_effects.get_mut(idx).map(|effect| effect.take());
    }

    pub fn ff_effect(&mut self, idx: usize) -> Option<&mut Effect> {
        self.ff_effects.get_mut(idx).and_then(|effect| effect.as_mut())
    }

    pub fn max_ff_effects(&self) -> usize {
        self.inner.max_ff_effects()
    }

    pub fn is_ff_supported(&self) -> bool {
        self.inner.is_ff_supported()
    }

    pub fn set_ff_gain(&mut self, gain: u16) {
        self.inner.set_ff_gain(gain)
    }
}

impl AsInner<platform::Gamepad> for Gamepad {
    fn as_inner(&self) -> &platform::Gamepad {
        &self.inner
    }

    fn as_inner_mut(&mut self) -> &mut platform::Gamepad {
        &mut self.inner
    }
}

pub trait GamepadImplExt {
    fn from_inner_status(inner: platform::Gamepad, status: Status) -> Self;
    fn state_mut(&mut self) -> &mut GamepadState;
    fn status_mut(&mut self) -> &mut Status;
}

impl GamepadImplExt for Gamepad {
    fn from_inner_status(inner: platform::Gamepad, status: Status) -> Self {
        Self::new(inner, status)
    }

    fn state_mut(&mut self) -> &mut GamepadState {
        &mut self.state
    }

    fn status_mut(&mut self) -> &mut Status {
        &mut self.status
    }
}

#[derive(Debug)]
pub struct Effect {
    inner: platform::Effect,
}

impl Effect {
    fn new(gamepad: &Gamepad, data: EffectData) -> Option<Self> {
        platform::Effect::new(&gamepad.inner, data).map(|effect| Effect { inner: effect })
    }

    pub fn upload(&mut self, data: EffectData) -> Option<()> {
        self.inner.upload(data)
    }

    pub fn play(&mut self, n: u16) {
        self.inner.play(n)
    }

    pub fn stop(&mut self) {
        self.inner.stop()
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct GamepadState {
    // sticks
    pub right_stick: (f32, f32),
    pub left_stick: (f32, f32),
    pub z: (f32, f32),
    pub btn_left_thumb: bool,
    pub btn_right_thumb: bool,
    // triggers
    pub right_trigger: f32,
    pub right_trigger2: f32,
    pub left_trigger: f32,
    pub left_trigger2: f32,
    // action pad
    pub btn_south: bool,
    pub btn_east: bool,
    pub btn_north: bool,
    pub btn_west: bool,
    pub btn_c: bool,
    pub btn_z: bool,
    // menu pad
    pub btn_select: bool,
    pub btn_start: bool,
    pub btn_mode: bool,
    // dpad
    pub btn_dpad_down: bool,
    pub btn_dpad_left: bool,
    pub btn_dpad_up: bool,
    pub btn_dpad_right: bool,
}

impl GamepadState {
    pub fn new() -> Self {
        unsafe { mem::zeroed() }
    }

    pub fn set_btn(&mut self, btn: Button, val: bool) {
        match btn {
            Button::South => self.btn_south = val,
            Button::East => self.btn_east = val,
            Button::North => self.btn_north = val,
            Button::West => self.btn_west = val,
            Button::C => self.btn_c = val,
            Button::Z => self.btn_z = val,

            Button::LeftTrigger => self.left_trigger = if val { 1.0 } else { 0.0 },
            Button::LeftTrigger2 => self.left_trigger2 = if val { 1.0 } else { 0.0 },
            Button::RightTrigger => self.right_trigger = if val { 1.0 } else { 0.0 },
            Button::RightTrigger2 => self.right_trigger2 = if val { 1.0 } else { 0.0 },

            Button::Select => self.btn_select = val,
            Button::Start => self.btn_start = val,
            Button::Mode => self.btn_mode = val,

            Button::LeftThumb => self.btn_left_thumb = val,
            Button::RightThumb => self.btn_right_thumb = val,

            Button::DPadUp => self.btn_dpad_up = val,
            Button::DPadDown => self.btn_dpad_down = val,
            Button::DPadRight => self.btn_dpad_right = val,
            Button::DPadLeft => self.btn_dpad_left = val,

            Button::Unknow => (),
        };
    }

    pub fn set_axis(&mut self, axis: Axis, val: f32) {
        match axis {
            Axis::LeftStickX => self.left_stick.0 = val,
            Axis::LeftStickY => self.left_stick.1 = val,
            Axis::LeftZ => self.z.0 = val,
            Axis::RightStickX => self.right_stick.0 = val,
            Axis::RightStickY => self.right_stick.1 = val,
            Axis::RightZ => self.z.1 = val,
            Axis::LeftTrigger => self.left_trigger = val,
            Axis::LeftTrigger2 => self.left_trigger2 = val,
            Axis::RightTrigger => self.right_trigger = val,
            Axis::RightTrigger2 => self.right_trigger2 = val,
        };
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Status {
    Connected,
    Disconnected,
    NotObserved,
}

pub struct EventIterator<'a> {
    inner: platform::EventIterator<'a>,
}

impl<'a> Iterator for EventIterator<'a> {
    type Item = (usize, Event);

    fn next(&mut self) -> Option<(usize, Event)> {
        self.inner.next()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    ButtonPressed(Button),
    ButtonReleased(Button),
    AxisChanged(Axis, f32),
    Connected,
    Disconnected,
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
    LeftTrigger = BTN_LT,
    LeftTrigger2 = BTN_LT2,
    RightTrigger = BTN_RT,
    RightTrigger2 = BTN_RT2,
    // Menu Pad
    Select = BTN_SELECT,
    Start = BTN_START,
    Mode = BTN_MODE,
    // Sticks
    LeftThumb = BTN_LTHUMB,
    RightThumb = BTN_RTHUMB,
    // D-Pad
    DPadUp = BTN_DPAD_UP,
    DPadDown = BTN_DPAD_DOWN,
    DPadLeft = BTN_DPAD_LEFT,
    DPadRight = BTN_DPAD_RIGHT,

    Unknow,
}

impl Default for Button {
    fn default() -> Self { Button::Unknow }
}

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Axis {
    LeftStickX = AXIS_LSTICKX,
    LeftStickY = AXIS_LSTICKY,
    LeftZ = AXIS_LEFTZ,
    RightStickX = AXIS_RSTICKX,
    RightStickY = AXIS_RSTICKY,
    RightZ = AXIS_RIGHTZ,
    LeftTrigger = AXIS_LT,
    LeftTrigger2 = AXIS_LT2,
    RightTrigger = AXIS_RT,
    RightTrigger2 = AXIS_RT2,
}

