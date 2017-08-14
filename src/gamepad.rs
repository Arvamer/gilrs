// Copyright 2016 GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use AsInner;
use constants::*;
use ff::Error as FfError;
use ff::server::{self, Message};
use mapping::{MappingData, MappingError};
use platform;

use uuid::Uuid;

use std::f32::NAN;
use std::ops::{Index, IndexMut};
use std::sync::mpsc::Sender;

/// Main object responsible of managing gamepads.
///
/// # Event loop
///
/// All interesting actions like button was pressed or new controller was connected are represented
/// by tuple `(usize, `[`Event`](enum.Event.html)`)`. You should call `poll_events()` method once in
/// your event loop and then iterate over all available events.
///
/// ```
/// use gilrs::{Gilrs, EventType, Button};
///
/// let mut gilrs = Gilrs::new();
///
/// // Event loop
/// loop {
///     for event in gilrs.poll_events() {
///         match event {
///             (id, EventType::ButtonPressed(Button::South, _)) => {
///                 println!("Player {}: jump!", id + 1)
///             }
///             (id, EventType::Disconnected) => println!("We lost player {}", id + 1),
///             _ => (),
///         };
///     }
///     # break;
/// }
/// ```
///
/// Additionally, every time you use `poll_events()`, cached gamepad state is updated. Use
/// `gamepad(usize)` method or index operator to borrow gamepad and then `state()`,
/// `is_pressed(Button)` or `value(Axis)` to examine gamepad's state. See
/// [`Gamepad`](struct.Gamepad.html) for more info.
#[derive(Debug)]
pub struct Gilrs {
    inner: platform::Gilrs,
    next_id: usize,
    tx: Sender<Message>,
}

impl Gilrs {
    /// Creates new `Gilrs`.
    pub fn new() -> Self {
        let gilrs = Gilrs {
            inner: platform::Gilrs::new(),
            next_id: 0,
            tx: server::init(),
        };
        gilrs.create_ff_devices();
        gilrs
    }

    /// Creates new `Gilrs` and add content of `sdl_mapping` to internal database. Each mapping
    /// should be in separate line. Lines that does not start from UUID are ignored.
    ///
    /// This function does not check validity of mappings.
    pub fn with_mappings(sdl_mapping: &str) -> Self {
        Gilrs {
            inner: platform::Gilrs::with_mappings(sdl_mapping),
            next_id: 0,
            tx: server::init(),
        }
    }

    fn create_ff_devices(&self) {
        for (id, gp) in self.gamepads().filter(|&(_, g)| g.is_ff_supported()).map(
            |(id, g)| (id, g.inner.ff_device()),
        )
        {
            if let Some(device) = gp {
                let _ = self.tx.send(Message::Open { id: id, device: device });
            }
        }
    }

    /// Creates iterator over available events. See [`EventType`](enum.EventType.html) for more
    /// information.
    pub fn poll_events(&mut self) -> EventIterator {
        EventIterator { gilrs: self }
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

    /// Returns iterator over all connected gamepads and their ids.
    ///
    /// ```
    /// # let gilrs = gilrs::Gilrs::new();
    /// for (id, gamepad) in gilrs.gamepads() {
    ///     assert!(gamepad.is_connected());
    ///     println!("Gamepad with id {} and name {} is connected",
    ///              id, gamepad.name());
    /// }
    /// ```
    pub fn gamepads(&self) -> ConnectedGamepadsIterator {
        ConnectedGamepadsIterator(self, 0)
    }

    /// Returns iterator over all connected gamepads and their ids.
    ///
    /// ```
    /// # let mut gilrs = gilrs::Gilrs::new();
    /// for (id, gamepad) in gilrs.gamepads_mut() {
    ///     assert!(gamepad.is_connected());
    ///     println!("Gamepad with id {} and name {} is connected",
    ///              id, gamepad.name());
    /// }
    /// ```
    pub fn gamepads_mut(&mut self) -> ConnectedGamepadsMutIterator {
        ConnectedGamepadsMutIterator(self, 0)
    }

    /// Returns a reference to connected gamepad or `None`.
    pub fn connected_gamepad(&self, id: usize) -> Option<&Gamepad> {
        let gp = self.inner.gamepad(id);
        if gp.is_connected() { Some(gp) } else { None }
    }

    /// Returns a mutable reference to connected gamepad or `None`.
    pub fn connected_gamepad_mut(&mut self, id: usize) -> Option<&mut Gamepad> {
        let gp = self.inner.gamepad_mut(id);
        if gp.is_connected() { Some(gp) } else { None }
    }

    pub fn set_listener_position<Vec3: Into<[f32; 3]>>(
        &self,
        idx: usize,
        position: Vec3,
    ) -> Result<(), FfError> {
        if !self.gamepad(idx).is_ff_supported() {
            Err(FfError::FfNotSupported(idx))
        } else {
            let _ = self.tx.send(Message::SetListenerPosition {
                id: idx,
                position: position.into(),
            });
            Ok(())
        }
    }

    pub(crate) fn ff_sender(&self) -> &Sender<Message> {
        &self.tx
    }

    pub(crate) fn next_ff_id(&mut self) -> usize {
        // TODO: reuse free ids
        let id = self.next_id;
        self.next_id = match self.next_id.checked_add(1) {
            Some(x) => x,
            None => panic!("Failed to assign ID to new effect"),
        };
        id
    }
}

impl Index<usize> for Gilrs {
    type Output = Gamepad;

    fn index(&self, idx: usize) -> &Gamepad {
        self.gamepad(idx)
    }
}

impl IndexMut<usize> for Gilrs {
    fn index_mut(&mut self, idx: usize) -> &mut Gamepad {
        self.gamepad_mut(idx)
    }
}

/// Iterator over all connected gamepads.
pub struct ConnectedGamepadsIterator<'a>(&'a Gilrs, usize);

impl<'a> Iterator for ConnectedGamepadsIterator<'a> {
    type Item = (usize, &'a Gamepad);

    fn next(&mut self) -> Option<(usize, &'a Gamepad)> {
        loop {
            if self.1 == self.0.inner.last_gamepad_hint() {
                return None;
            }

            if let Some(gp) = self.0.connected_gamepad(self.1) {
                let idx = self.1;
                self.1 += 1;
                return Some((idx, gp));
            }

            self.1 += 1;
        }
    }
}

/// Iterator over all connected gamepads.
pub struct ConnectedGamepadsMutIterator<'a>(&'a mut Gilrs, usize);

impl<'a> Iterator for ConnectedGamepadsMutIterator<'a> {
    type Item = (usize, &'a mut Gamepad);

    fn next(&mut self) -> Option<(usize, &'a mut Gamepad)> {
        loop {
            if self.1 == self.0.inner.last_gamepad_hint() {
                return None;
            }

            if let Some(gp) = self.0.connected_gamepad_mut(self.1) {
                let idx = self.1;
                self.1 += 1;
                let gp = unsafe { &mut *(gp as *mut _) };
                return Some((idx, gp));
            }

            self.1 += 1;
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
    threshold: Deadzones,
}

impl Gamepad {
    fn new(gamepad: platform::Gamepad, status: Status, threshold: Deadzones) -> Self {
        Gamepad {
            inner: gamepad,
            state: GamepadState::new(),
            status: status,
            // Effect doesn't implement Clone so we can't use vec! macro.
            threshold: threshold,
        }
    }

    /// Returns gamepad's name.
    pub fn name(&self) -> &str {
        self.inner.name()
    }

    /// Returns gamepad's UUID.
    pub fn uuid(&self) -> Uuid {
        self.inner.uuid()
    }

    /// Returns cached gamepad state.
    ///
    /// Every time you use `Gilrs::poll_events()` gamepad state is updated. You can use it to know
    /// if some button is pressed or to get axis's value.
    ///
    /// ```
    /// use gilrs::{Gilrs, Button, Axis};
    ///
    /// let mut gilrs = Gilrs::new();
    ///
    /// loop {
    ///     for _ in gilrs.poll_events() {}
    ///
    ///     println!("Start: {}, Left Stick X: {}",
    ///              gilrs[0].is_pressed(Button::Start),
    ///              gilrs[0].value(Axis::LeftStickX));
    ///     # break;
    /// }
    /// ```
    pub fn state(&self) -> &GamepadState {
        &self.state
    }

    /// Returns current gamepad's status, which can be `Connected`, `Disconnected` or `NotObserved`.
    /// Only connected gamepads generate events. Disconnected gamepads retain their name and UUID.
    /// Cached state of disconnected and not observed gamepads is 0 (false for buttons and 0.0 for
    /// axis) and all actions preformed on such gamepad are no-op.
    pub fn status(&self) -> Status {
        self.status
    }

    /// Returns true if gamepad is connected.
    pub fn is_connected(&self) -> bool {
        self.status == Status::Connected
    }

    /// Examines cached gamepad state to check if given button is pressed. If `btn` can also be
    /// represented by axis returns true if value is not equal to 0.0. Always returns `false` for
    /// `Button::Unknown`.
    pub fn is_pressed(&self, btn: Button) -> bool {
        self.state.is_pressed(btn)
    }

    /// Examines cached gamepad state to check axis's value. If `axis` is represented by button on
    /// device it value is 0.0 if button is not pressed or 1.0 if is pressed. Returns `NaN` for
    /// `Axis::Unknown`.
    pub fn value(&self, axis: Axis) -> f32 {
        self.state.value(axis)
    }

    /// Returns device's power supply state. See [`PowerInfo`](enum.PowerInfo.html) for details.
    pub fn power_info(&self) -> PowerInfo {
        self.inner.power_info()
    }

    /// Returns source of gamepad mapping. Can be used to filter gamepads which do not provide
    /// unified controller layout.
    ///
    /// ```
    /// use gilrs::MappingSource;
    /// # let mut gilrs = gilrs::Gilrs::new();
    ///
    /// for (_, gamepad) in gilrs.gamepads().filter(
    ///     |gp| gp.1.mapping_source() != MappingSource::None)
    /// {
    ///     println!("{} is ready to use!", gamepad.name());
    /// }
    pub fn mapping_source(&self) -> MappingSource {
        self.inner.mapping_source()
    }

    /// Sets gamepad's mapping and returns SDL2 representation of them. Returned mappings may not be
    /// compatible with SDL2 - if it is important, use
    /// [`set_mapping_strict()`](#method.set_mapping_strict).
    ///
    /// The `name` argument can be a string slice with custom gamepad name or `None`. If `None`,
    /// gamepad name reported by driver will be used.
    ///
    /// # Errors
    ///
    /// This function return error if `name` contains comma, `mapping` have axis and button entry
    /// for same element (for example `Axis::LetfTrigger` and `Button::LeftTrigger`) or gamepad does
    /// not have any element with `NativeEvCode` used in mapping. `Button::Unknown` and
    /// `Axis::Unknown` are not allowd as keys to `mapping` – in this case,
    /// `MappingError::UnknownElement` is returned.
    ///
    /// Error is also returned if this function is not implemented or gamepad is not connected.
    ///
    /// # Example
    ///
    /// ```
    /// use gilrs::{Mapping, Button};
    ///
    /// # let mut gilrs = gilrs::Gilrs::new();
    /// let mut data = Mapping::new();
    /// data[Button::South] = 213;
    /// // …
    ///
    /// // or `match gilrs[0].set_mapping(&data, None) {`
    /// match gilrs[0].set_mapping(&data, "Custom name") {
    ///     Ok(sdl) => println!("SDL2 mapping: {}", sdl),
    ///     Err(e) => println!("Failed to set mapping: {}", e),
    /// };
    /// ```
    ///
    /// Example with `MappingError::DuplicatedEntry`:
    ///
    /// ```no_run
    /// use gilrs::{Mapping, Button, Axis, MappingError};
    ///
    /// # let mut gilrs = gilrs::Gilrs::new();
    /// let mut data = Mapping::new();
    /// data[Button::RightTrigger2] = 2;
    /// data[Axis::RightTrigger2] = 2;
    ///
    /// assert_eq!(gilrs[0].set_mapping(&data, None), Err(MappingError::DuplicatedEntry));
    /// ```
    ///
    /// See also `examples/mapping.rs`.
    pub fn set_mapping<'a, O: Into<Option<&'a str>>>(
        &mut self,
        mapping: &MappingData,
        name: O,
    ) -> Result<String, MappingError> {
        self.inner.set_mapping(mapping, false, name.into())
    }

    /// Similar to [`set_mapping()`](#method.set_mapping) but returned string should be compatible
    /// with SDL2.
    ///
    /// # Errors
    ///
    /// Returns `MappingError::NotSdl2Compatible` if `mapping` have an entry for `Button::{C, Z}`
    /// or `Axis::{LeftZ, RightZ}`.
    pub fn set_mapping_strict<'a, O: Into<Option<&'a str>>>(
        &mut self,
        mapping: &MappingData,
        name: O,
    ) -> Result<String, MappingError> {
        if mapping.button(Button::C).is_some() || mapping.button(Button::Z).is_some() ||
            mapping.axis(Axis::LeftZ).is_some() || mapping.axis(Axis::RightZ).is_some()
        {
            Err(MappingError::NotSdl2Compatible)
        } else {
            self.inner.set_mapping(mapping, true, name.into())
        }
    }

    /// Returns true if force feedback is supported by device.
    pub fn is_ff_supported(&self) -> bool {
        self.inner.is_ff_supported()
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
    fn from_inner_status(inner: platform::Gamepad, status: Status, threshold: Deadzones) -> Self;
}

impl GamepadImplExt for Gamepad {
    fn from_inner_status(inner: platform::Gamepad, status: Status, threshold: Deadzones) -> Self {
        Self::new(inner, status, threshold)
    }
}

/// Cached state of gamepad's buttons and axes.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
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
    /// Creates new `GamepadState` with all values zeroed.
    pub fn new() -> Self {
        Default::default()
    }

    /// Sets new value for given button.
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

            Button::Unknown => (),
        };
    }

    /// Sets new value for given axis.
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
            Axis::Unknown => (),
        };
    }

    /// Examines cached gamepad state to check if given button is pressed. If `btn` can also be
    /// represented by axis returns true if value is not equal to 0.0. Always returns `false` for
    /// `Button::Unknown`.
    pub fn is_pressed(&self, btn: Button) -> bool {
        match btn {
            Button::South => self.btn_south,
            Button::East => self.btn_east,
            Button::North => self.btn_north,
            Button::West => self.btn_west,
            Button::C => self.btn_c,
            Button::Z => self.btn_z,

            Button::LeftTrigger => self.left_trigger != 0.0,
            Button::LeftTrigger2 => self.left_trigger2 != 0.0,
            Button::RightTrigger => self.right_trigger != 0.0,
            Button::RightTrigger2 => self.right_trigger2 != 0.0,

            Button::Select => self.btn_select,
            Button::Start => self.btn_start,
            Button::Mode => self.btn_mode,

            Button::LeftThumb => self.btn_left_thumb,
            Button::RightThumb => self.btn_right_thumb,

            Button::DPadUp => self.btn_dpad_up,
            Button::DPadDown => self.btn_dpad_down,
            Button::DPadRight => self.btn_dpad_right,
            Button::DPadLeft => self.btn_dpad_left,

            Button::Unknown => false,
        }
    }

    /// Examines cached gamepad state to check axis's value. If `axis` is represented by button on
    /// device it value is 0.0 if button is not pressed or 1.0 if is pressed. Returns `NaN` for
    /// `Axis::Unknown`.
    pub fn value(&self, axis: Axis) -> f32 {
        match axis {
            Axis::LeftStickX => self.left_stick.0,
            Axis::LeftStickY => self.left_stick.1,
            Axis::LeftZ => self.z.0,
            Axis::RightStickX => self.right_stick.0,
            Axis::RightStickY => self.right_stick.1,
            Axis::RightZ => self.z.1,
            Axis::LeftTrigger => self.left_trigger,
            Axis::LeftTrigger2 => self.left_trigger2,
            Axis::RightTrigger => self.right_trigger,
            Axis::RightTrigger2 => self.right_trigger2,
            Axis::Unknown => NAN, // or return 0.0?
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
/// Status of gamepad's connection.
///
/// Only connected gamepads generate events. Disconnected gamepads retain their name and UUID.
/// Cached state of disconnected and not observed gamepads is 0 (false for buttons and 0.0 for
/// axis) and all actions preformed on such gamepad are no-op.
pub enum Status {
    Connected,
    Disconnected,
    NotObserved,
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Deadzones {
    pub right_stick: f32,
    pub left_stick: f32,
    pub left_z: f32,
    pub right_z: f32,
    pub right_trigger: f32,
    pub right_trigger2: f32,
    pub left_trigger: f32,
    pub left_trigger2: f32,
}

impl Deadzones {
    #[allow(dead_code)]
    pub fn set(&mut self, axis: Axis, val: f32) {
        match axis {
            Axis::LeftStickX => self.left_stick = val,
            Axis::LeftStickY => self.left_stick = val,
            Axis::LeftZ => self.left_z = val,
            Axis::RightStickX => self.right_stick = val,
            Axis::RightStickY => self.right_stick = val,
            Axis::RightZ => self.right_z = val,
            Axis::LeftTrigger => self.left_trigger = val,
            Axis::LeftTrigger2 => self.left_trigger2 = val,
            Axis::RightTrigger => self.right_trigger = val,
            Axis::RightTrigger2 => self.right_trigger2 = val,
            Axis::Unknown => (),
        };
    }

    pub fn get(&self, axis: Axis) -> f32 {
        match axis {
            Axis::LeftStickX => self.left_stick,
            Axis::LeftStickY => self.left_stick,
            Axis::LeftZ => self.left_z,
            Axis::RightStickX => self.right_stick,
            Axis::RightStickY => self.right_stick,
            Axis::RightZ => self.right_z,
            Axis::LeftTrigger => self.left_trigger,
            Axis::LeftTrigger2 => self.left_trigger2,
            Axis::RightTrigger => self.right_trigger,
            Axis::RightTrigger2 => self.right_trigger2,
            Axis::Unknown => 0.05,
        }
    }
}

/// Platform specific event code.
///
/// Meaning of specific codes can vary not only between platforms but also between different
/// devices. Context is also important - axis with code 2 is something totally different than button
/// with code 2.
///
/// **DPad is often represented as 2 axis, not 4 buttons.** So if you get event `(0,
/// Button::DPadDown, 4)`, you can not be sure if 4 is button or axis. On Linux you can assume that
/// if event code is smaller than 0x100 it's either keyboard key or axis.
pub type NativeEvCode = u16;

/// Iterator over gamepads events
pub struct EventIterator<'a> {
    gilrs: &'a mut Gilrs,
}

impl<'a> Iterator for EventIterator<'a> {
    type Item = (usize, EventType);

    fn next(&mut self) -> Option<(usize, EventType)> {
        match self.gilrs.inner.next_event() {
            Some((id, ev)) => {
                let mut maybe_disconnected = None;
                {
                    let gamepad = self.gilrs.gamepad_mut(id);
                    match ev {
                        EventType::ButtonPressed(btn, _) => gamepad.state.set_btn(btn, true),
                        EventType::ButtonReleased(btn, _) => gamepad.state.set_btn(btn, false),
                        EventType::AxisChanged(axis, val, native_ev_code) => {
                            let val = match axis {
                                Axis::LeftStickX => {
                                    apply_deadzone(
                                        val,
                                        gamepad.value(Axis::LeftStickY),
                                        gamepad.threshold.left_stick,
                                    ).0
                                }
                                Axis::LeftStickY => {
                                    apply_deadzone(
                                        val,
                                        gamepad.value(Axis::LeftStickX),
                                        gamepad.threshold.left_stick,
                                    ).0
                                }
                                Axis::RightStickX => {
                                    apply_deadzone(
                                        val,
                                        gamepad.value(Axis::RightStickY),
                                        gamepad.threshold.right_stick,
                                    ).0
                                }
                                Axis::RightStickY => {
                                    apply_deadzone(
                                        val,
                                        gamepad.value(Axis::RightStickX),
                                        gamepad.threshold.right_stick,
                                    ).0
                                }
                                axis => apply_deadzone(val, 0.0, gamepad.threshold.get(axis)).0,
                            };
                            if gamepad.value(axis) != val {
                                gamepad.state.set_axis(axis, val);
                                return Some(
                                    (id, EventType::AxisChanged(axis, val, native_ev_code)),
                                );
                            } else {
                                return None;
                            }
                        }
                        EventType::Connected => gamepad.status = Status::Connected,
                        EventType::Disconnected => {
                            gamepad.status = Status::Disconnected;
                            maybe_disconnected = Some(id);
                        }
                    };
                }
                if let Some(id) = maybe_disconnected {
                    let _ = self.gilrs.tx.send(Message::Close { id });
                }
                Some((id, ev))
            }
            None => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Gamepad event.
pub enum EventType {
    /// Some button on gamepad has been pressed.
    ButtonPressed(Button, NativeEvCode),
    /// Previously pressed button has been released.
    ButtonReleased(Button, NativeEvCode),
    /// Value of axis has changed. Value can be in range [-1.0, 1.0] for sticks and [0.0, 1.0] for
    /// triggers.
    AxisChanged(Axis, f32, NativeEvCode),
    /// Gamepad has been connected. If gamepad's UUID doesn't match one of disconnected gamepads,
    /// newly connected gamepad will get new ID.
    Connected,
    /// Gamepad has been disconnected. Disconnected gamepad will not generate any new events.
    Disconnected,
}

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq)]
/// Gamepad's elements which state can be represented by `bool`.
///
/// ![Controller layout](https://arvamer.gitlab.io/gilrs/img/controller.svg)
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

    Unknown = BTN_UNKNOWN,
}

impl Button {
    pub fn is_action(self) -> bool {
        use Button::*;
        match self {
            South | East | North | West | C | Z => true,
            _ => false,
        }
    }

    pub fn is_trigger(self) -> bool {
        use Button::*;
        match self {
            LeftTrigger | LeftTrigger2 | RightTrigger | RightTrigger2 => true,
            _ => false,
        }
    }

    pub fn is_menu(self) -> bool {
        use Button::*;
        match self {
            Select | Start | Mode => true,
            _ => false,
        }
    }

    pub fn is_stick(self) -> bool {
        use Button::*;
        match self {
            LeftThumb | RightThumb => true,
            _ => false,
        }
    }

    pub fn is_dpad(self) -> bool {
        use Button::*;
        match self {
            DPadUp | DPadDown | DPadLeft | DPadRight => true,
            _ => false,
        }
    }
}

impl Default for Button {
    fn default() -> Self {
        Button::Unknown
    }
}

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq)]
/// Gamepad's elements which state can be represented by `f32`.
///
/// ![Controller layout](https://arvamer.gitlab.io/gilrs/img/controller.svg)
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
    Unknown = AXIS_UNKNOWN,
}

impl Axis {
    pub fn is_stick(self) -> bool {
        use Axis::*;
        match self {
            LeftStickX | LeftStickY | RightStickX | RightStickY => true,
            _ => false,
        }
    }

    pub fn is_trigger(self) -> bool {
        use Axis::*;
        match self {
            LeftTrigger | LeftTrigger2 | RightTrigger | RightTrigger2 | LeftZ | RightZ => true,
            _ => false,
        }
    }
}


/// State of device's power supply.
///
/// Battery level is reported as integer between 0 and 100.
///
/// ## Example
///
/// ```
/// use gilrs::PowerInfo;
/// # let gilrs = gilrs::Gilrs::new();
///
/// match gilrs.gamepad(0).power_info() {
///     PowerInfo::Discharging(lvl) if lvl <= 10 => println!("Low battery level, you should \
///                                                           plug your gamepad"),
///     _ => (),
/// };
/// ```
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PowerInfo {
    /// Failed to determine power status.
    Unknown,
    /// Device doesn't have battery.
    Wired,
    /// Device is running on the battery.
    Discharging(u8),
    /// Battery is charging.
    Charging(u8),
    /// Battery is charged.
    Charged,
}

/// Source of gamepad mappings.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MappingSource {
    /// Gamepad uses SDL mappings.
    SdlMappings,
    /// Gamepad does not use any mappings but driver should provide unified controller layout.
    Driver,
    /// Gamepad does not use any mappings and most gamepad events will probably be `Button::Unknown`
    /// or `Axis::Unknown`
    None,
}

pub fn apply_deadzone(x: f32, y: f32, threshold: f32) -> (f32, f32) {
    let magnitude = (x * x + y * y).sqrt();
    if magnitude <= threshold {
        (0.0, 0.0)
    } else {
        let norm = ((magnitude - threshold) / (1.0 - threshold)) / magnitude;
        (x * norm, y * norm)
    }
}
