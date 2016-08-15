use platform;
use std::mem;
use constants::*;
use ff::EffectData;
use uuid::Uuid;
use AsInner;

/// Main object responsible of managing gamepads.
///
/// # Event loop
///
/// All interesting actions like button was pressed or new controller was connected are represented
/// by tuple `(usize, `[`Event`](enum.Event.html)`)`. You should call `pool_events()` method once in
/// your event loop and then iterate over all available events.
///
/// ```
/// use gilrs::{Gilrs, Event, Button};
///
/// let mut gilrs = Gilrs::new();
///
/// // Event loop
/// loop {
///     for event in gilrs.pool_events() {
///         match event {
///             (id, Event::ButtonPressed(Button::South)) => println!("Player {}: jump!", id + 1),
///             (id, Event::Disconnected) => println!("We lost player {}", id + 1),
///             _ => (),
///         };
///     }
///     # break;
/// }
/// ```
///
/// Additionally, every time you use `pool_events()`, cached gamepad state is updated. Use
/// `gamepad(usize)` method to borrow gamepad and then `state()`, `is_btn_pressed(Button)` or
/// `axis_val(Axis)` to examine gamepad's state. See [`Gamepad`](struct.Gamepad.html) for more
/// info.
#[derive(Debug)]
pub struct Gilrs {
    inner: platform::Gilrs,
}

impl Gilrs {
    /// Creates new `Gilrs`.
    pub fn new() -> Self {
        Gilrs { inner: platform::Gilrs::new() }
    }

    /// Creates iterator over available events. Iterator item's is `(usize, Event)` where usize is
    /// id of gamepad that generated event. See struct level documentation for example.
    pub fn pool_events(&mut self) -> EventIterator {
        EventIterator { inner: self.inner.pool_events() }
    }

    /// Borrow gamepad with given id. This method always return reference to some gamepad, even if
    /// it was disconnected or never observed. If gamepad's status is not equal to
    /// `Status::Connected` all actions preformed on it are no-op and all values in cached gamepad
    /// state are 0 (false for buttons and 0.0 for axes).
    pub fn gamepad(&self, id: usize) -> &Gamepad {
        self.inner.gamepad(id)
    }

    /// See `gamepad()`
    pub fn gamepad_mut(&mut self, id: usize) -> &mut Gamepad {
        self.inner.gamepad_mut(id)
    }

    /// Returns reference to connected gamepad or `None`.
    pub fn connected_gamepad(&self, id: usize) -> Option<&Gamepad> {
        let gp = self.inner.gamepad(id);
        if gp.is_connected() {
            Some(gp)
        } else {
            None
        }
    }

    /// Returns reference to connected gamepad or `None`.
    pub fn connected_gamepad_mut(&mut self, id: usize) -> Option<&mut Gamepad> {
        let mut gp = self.inner.gamepad_mut(id);
        if gp.is_connected() {
            Some(gp)
        } else {
            None
        }
    }
}

/// Represents game controller.
///
/// Using this struct you can access cached gamepad state, informations about gamepad such as name
/// or UUID and manage force feedback effects.
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

    /// Returns gamepad's name.
    pub fn name(&self) -> &String {
        self.inner.name()
    }

    /// Returns gamepad's UUID.
    pub fn uuid(&self) -> Uuid {
        self.inner.uuid()
    }

    /// Returns cached gamepad state.
    ///
    /// Every time you use `Gilrs::pool_events()` gamepad state is updated. You can use it to know
    /// if some button is pressed or to get axis's value.
    ///
    /// ```
    /// use gilrs::{Gilrs, Button, Axis};
    ///
    /// let mut gilrs = Gilrs::new();
    ///
    /// loop {
    ///     for _ in gilrs.pool_events() {}
    ///
    ///     println!("Start: {}, Left Stick X: {}",
    ///              gilrs.gamepad(0).is_btn_pressed(Button::Start),
    ///              gilrs.gamepad(0).axis_val(Axis::LeftStickX));
    ///     # break;
    /// }
    /// ```
    pub fn state(&self) -> &GamepadState {
        &self.state
    }

    /// Returns current gamepad's status, which can be `Connected`, `Disconnected` or `NotObserved`.
    /// Only connected gamepads generate events. Disconnected gamepads retain their name and UUID.
    /// Cached state of disconnected and not observed gamepads is 0 (false for buttons and 0.0 for
    /// axis) and all acions preformed on such gamepad are no-op.
    pub fn status(&self) -> Status {
        self.status
    }

    /// Returns true if gamepad is connected.
    pub fn is_connected(&self) -> bool {
        self.status == Status::Connected
    }

    /// Examines cached gamepad state to check if given button is pressed. If `btn` can also be
    /// represented by axis returns true if value is not equal to 0.0.
    pub fn is_btn_pressed(&self, btn: Button) -> bool {
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

    /// Examines cached gamepad state to check axis's value. If `axis` is represented by button on
    /// device it value is 0.0 if button is not pressed or 1.0 if is pressed.
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

    /// Creates and uploads new force feedback effect using `data`. This function will fail if device
    /// doesn't have space for new effect or doesn't support requested effect. Returns effect's
    /// index.
    ///
    /// ```rust,no_run
    /// use gilrs::ff::EffectData;
    /// use gilrs::Gilrs;
    ///
    /// let mut gilrs = Gilrs::new();
    ///
    /// let mut effect = EffectData::default();
    /// effect.period = 1000;
    /// effect.magnitude = 20000;
    /// effect.replay.length = 5000;
    /// effect.envelope.attack_length = 1000;
    /// effect.envelope.fade_length = 1000;
    ///
    /// let effect_idx = gilrs.gamepad_mut(0).add_ff_effect(effect).unwrap();
    /// gilrs.gamepad_mut(0).ff_effect(effect_idx).unwrap().play(1);
    /// ```
    // FIXME: Change return to Result and create appropriate error
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

    /// Drop effect stopping it. Use this function to make space for new effects.
    pub fn drop_ff_effect(&mut self, idx: usize) {
        self.ff_effects.get_mut(idx).map(|effect| effect.take());
    }

    /// Borrows mutable `Effect`.
    pub fn ff_effect(&mut self, idx: usize) -> Option<&mut Effect> {
        self.ff_effects.get_mut(idx).and_then(|effect| effect.as_mut())
    }

    /// Returns how many force feedback effects device can have.
    pub fn max_ff_effects(&self) -> usize {
        self.inner.max_ff_effects()
    }

    /// Returns true if force feedback is supported by device.
    pub fn is_ff_supported(&self) -> bool {
        self.inner.is_ff_supported()
    }

    /// Sets master gain for device.
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

impl Button {
    pub fn is_action(self) -> bool {
        use Button::*;
        match self {
            South | East | North | West | C | Z => true,
            _ => false
        }
    }

    pub fn is_trigger(self) -> bool {
        use Button::*;
        match self {
            LeftTrigger | LeftTrigger2 | RightTrigger | RightTrigger2 => true,
            _ => false
        }
    }

    pub fn is_menu(self) -> bool {
        use Button::*;
        match self {
            Select | Start | Mode => true,
            _ => false
        }
    }

    pub fn is_stick(self) -> bool {
        use Button::*;
        match self {
            LeftThumb | RightThumb => true,
            _ => false
        }
    }

    pub fn is_dpad(self) -> bool {
        use Button::*;
        match self {
            DPadUp | DPadDown | DPadLeft | DPadRight => true,
            _ => false
        }
    }

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

impl Axis {
    pub fn is_stick(self) -> bool {
        use Axis::*;
        match self {
            LeftStickX | LeftStickY | RightStickX | RightStickY => true,
            _ => false
        }
    }

    pub fn is_trigger(self) -> bool {
        use Axis::*;
        match self {
            LeftTrigger | LeftTrigger2 | RightTrigger | RightTrigger2 | LeftZ | RightZ => true,
            _ => false
        }
    }
}
