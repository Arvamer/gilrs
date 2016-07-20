use platform;
use std::mem;
use constants::*;
use ff::EffectData;
use uuid::Uuid;

#[derive(Debug)]
pub struct Gilrs {
    inner: platform::Gilrs,
    gamepads: Vec<Gamepad>,
    // Use it to out of bound access
    not_observed_gp: Gamepad,
}

impl Gilrs {
    pub fn new() -> Self {
        let mut inner = platform::Gilrs::new();
        let gamepads = inner.gamepads.drain(0..)
            .map(|gp| Gamepad::new(gp, Status::Connected))
            .collect();
        Gilrs {
            inner: inner,
            gamepads: gamepads,
            not_observed_gp: Gamepad::new(platform::Gamepad::none(), Status::NotObserved),
        }
    }

    pub fn pool_events(&mut self) -> EventIterator {
        EventIterator(self, 0)
    }

    fn handle_hotplug(&mut self) -> Option<(usize, Event)> {
        self.inner.handle_hotplug().and_then(|(gamepad, status)| {
            match status {
                Status::Connected => Some((self.gamepad_connected(gamepad), Event::Connected)),
                Status::Disconnected => self.gamepad_disconnected(gamepad).map(|id| (id, Event::Disconnected)),
                Status::NotObserved => unreachable!(),
            }
        })
    }

    fn gamepad_connected(&mut self, gamepad: platform::Gamepad) -> usize {
        match self.gamepads.iter()
                           .position(|gp| gp.status == Status::Disconnected && gp.inner == gamepad) {
            Some(id) => {
                self.gamepads[id] = Gamepad::new(gamepad, Status::Connected);
                id
            }
            None => {
                self.gamepads.push(Gamepad::new(gamepad, Status::Connected));
                self.gamepads.len() - 1
            }
        }
    }

    fn gamepad_disconnected(&mut self, gamepad: platform::Gamepad) -> Option<usize> {
        self.gamepads.iter().position(|gp| gp.inner.eq_disconnect(&gamepad)).map(|id| {
            self.gamepads[id].inner.disconnect();
            self.gamepads[id].status = Status::Disconnected;
            id
        })
    }

    pub fn gamepad(&self, n: usize) -> &Gamepad {
        self.gamepads.get(n).unwrap_or(&self.not_observed_gp)
    }

    pub fn gamepad_mut(&mut self, n: usize) -> &mut Gamepad {
        self.gamepads.get_mut(n).unwrap_or(&mut self.not_observed_gp)
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
        &self.inner.name
    }

    pub fn uuid(&self) -> Uuid {
        self.inner.uuid
    }

    pub fn state(&self) -> &GamepadState {
        &self.state
    }

    pub fn status(&self) -> Status {
        self.status
    }

    pub fn is_pressed(&self, btn: Button) -> bool {
        match btn {
            Button::South => self.state.btn_south,
            Button::East => self.state.btn_east,
            Button::North => self.state.btn_north,
            Button::West => self.state.btn_west,
            Button::C => self.state.btn_c,
            Button::Z => self.state.btn_z,

            Button::LeftTrigger => self.state.left_trigger != 0.0,
            Button::LeftTrigger2 => self.state.left_trigger2 != 0.0,
            Button::RightTrigger => self.state.right_trigger != 0.0,
            Button::RightTrigger2 => self.state.right_trigger2 != 0.0,

            Button::Select => self.state.btn_select,
            Button::Start => self.state.btn_start,
            Button::Mode => self.state.btn_mode,

            Button::LeftThumb => self.state.btn_left_thumb,
            Button::RightThumb => self.state.btn_right_thumb,

            Button::DPadUp => self.state.dpad.0 > 0.0,
            Button::DPadDown => self.state.dpad.0 < 0.0,
            Button::DPadRight => self.state.dpad.1 > 0.0,
            Button::DPadLeft => self.state.dpad.1 < 0.0,

            Button::Unknow => false,
        }
    }

    pub fn axis_val(&self, axis: Axis) -> f32 {
        match axis {
            Axis::LeftStickX => self.state.left_stick.0,
            Axis::LeftStickY => self.state.left_stick.1,
            Axis::LeftZ => self.state.z.0,
            Axis::RightStickX => self.state.right_stick.0,
            Axis::RightStickY => self.state.right_stick.1,
            Axis::RightZ => self.state.z.1,
            Axis::DPadX => self.state.dpad.0,
            Axis::DPadY => self.state.dpad.1,
            Axis::LeftTrigger => self.state.left_trigger,
            Axis::LeftTrigger2 => self.state.left_trigger2,
            Axis::RightTrigger => self.state.right_trigger,
            Axis::RightTrigger2 => self.state.right_trigger2,
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
    pub dpad: (f32, f32),
}

impl GamepadState {
    pub fn new() -> Self {
        unsafe { mem::zeroed() }
    }

    fn set_btn(&mut self, btn: Button, val: bool) {
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

            Button::DPadUp => self.dpad.0 = if val { 1.0 } else { 0.0 },
            Button::DPadDown => self.dpad.0 = if val { -1.0 } else { 0.0 },
            Button::DPadRight => self.dpad.1 = if val { 1.0 } else { 0.0 },
            Button::DPadLeft => self.dpad.1 = if val { -1.0 } else { 0.0 },

            Button::Unknow => (),
        };
    }

    fn set_axis(&mut self, axis: Axis, val: f32) {
        match axis {
            Axis::LeftStickX => self.left_stick.0 = val,
            Axis::LeftStickY => self.left_stick.1 = val,
            Axis::LeftZ => self.z.0 = val,
            Axis::RightStickX => self.right_stick.0 = val,
            Axis::RightStickY => self.right_stick.1 = val,
            Axis::RightZ => self.z.1 = val,
            Axis::DPadX => self.dpad.0 = val,
            Axis::DPadY => self.dpad.1 = val,
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

pub struct EventIterator<'a>(&'a mut Gilrs, usize);

impl<'a> Iterator for EventIterator<'a> {
    type Item = (usize, Event);

    fn next(&mut self) -> Option<(usize, Event)> {
        loop {
            if let Some(ev) = self.0.handle_hotplug() {
                return Some(ev);
            }

            let mut gamepad = match self.0.gamepads.get_mut(self.1) {
                Some(gp) => gp,
                None => return None,
            };

            if gamepad.status != Status::Connected {
                continue;
            }

            match gamepad.inner.event() {
                None => {
                    self.1 += 1;
                    continue;
                }
                Some(ev) => {
                    match ev {
                        Event::ButtonPressed(btn) => gamepad.state.set_btn(btn, true),
                        Event::ButtonReleased(btn) => gamepad.state.set_btn(btn, false),
                        Event::AxisChanged(axis, val) => gamepad.state.set_axis(axis, val),
                        _ => unreachable!(),
                    }
                    return Some((self.1, ev));
                }
            }
        }
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
    DPadX = AXIS_DPADX,
    DPadY = AXIS_DPADY,
    LeftTrigger = AXIS_LT,
    LeftTrigger2 = AXIS_LT2,
    RightTrigger = AXIS_RT,
    RightTrigger2 = AXIS_RT2,
}

