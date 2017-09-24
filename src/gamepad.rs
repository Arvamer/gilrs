// Copyright 2017 Mateusz Sieczko and other GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use AsInner;
use constants::*;
use ev::{AxisData, ButtonData, GamepadState};
use ff::Error as FfError;
use ff::server::{self, Message};
use mapping::{Mapping, MappingData, MappingDb, MappingError};
use platform;

use uuid::Uuid;

use std::ops::{Index, IndexMut};
use std::sync::mpsc::Sender;
use std::time::SystemTime;

/// Main object responsible of managing gamepads.
///
/// # Event loop
///
/// All interesting actions like button was pressed or new controller was connected are represented
/// by struct [`Event`](struct.Event.html). Use `next_event()` function to retrieve event from
/// queue.
///
/// ```
/// use gilrs::{Gilrs, Event, EventType, Button};
///
/// let mut gilrs = Gilrs::new();
///
/// // Event loop
/// loop {
///     while let Some(event) = gilrs.next_event() {
///         match event {
///             Event { id, event: EventType::ButtonPressed(Button::South, _), .. } => {
///                 println!("Player {}: jump!", id + 1)
///             }
///             Event { id, event: EventType::Disconnected, .. } => {
///                 println!("We lost player {}", id + 1)
///             }
///             _ => (),
///         };
///     }
///     # break;
/// }
/// ```
///
/// # Cached gamepad state
///
/// `Gilrs` also menage cached gamepad state. To update it, use `update(Event)` method. Updating is
/// not done automatically, because you probably want the state after filtered events (see
/// [`ev::filter`](ev/filter/index.html) module), not these from `event_next()`.
///
/// To access state you can use `Gamepad::state()` function. Gamepad also implement some state
/// related functions directly, see [`Gamepad`](struct.Gamepad.html) for more.
///
/// ## Counter
///
/// `Gilrs` has additional functionality, referred here as *counter*. The idea behind it is simple,
/// each time you end iteration of update loop, you call `Gilrs::inc()` which will increase
/// internal counter by one. When state of one if elements changes, value of counter is saved. When
/// checking state of one of elements you can tell exactly when this event happened. Timestamps are
/// not good solution here because they can tell you when *system* observed event, not when you
/// processed it. On the other hand, they are good when you want to implement key repeat or software
/// debouncing.
///
/// ```
/// use gilrs::{Gilrs, Button};
///
/// let mut gilrs = Gilrs::new();
///
/// loop {
///     while let Some(ev) = gilrs.next_event() {
///         gilrs.update(&ev);
///         // Do other things with event
///     }
///
///     if gilrs.gamepad(0).is_pressed(Button::DPadLeft) {
///         // go left
///     }
///
///     match gilrs.gamepad(0).button_data(Button::South) {
///         Some(d) if d.is_pressed() && d.counter() == gilrs.counter() => {
///             // jump
///         }
///         _ => ()
///     }
///
///     gilrs.inc();
/// #   break;
/// }
///
#[derive(Debug)]
pub struct Gilrs {
    inner: platform::Gilrs,
    next_id: usize,
    tx: Sender<Message>,
    counter: u64,
    mappings: MappingDb,
    default_filters: bool,
}

impl Gilrs {
    /// Creates new `Gilrs` with default settings. See [`GilrsBuilder`](struct.GilrsBuilder.html)
    /// for more details.
    pub fn new() -> Self {
        GilrsBuilder::new().build()
    }

    /// Returns next pending event.
    pub fn next_event(&mut self) -> Option<Event> {
        use ev::filter::{axis_dpad_to_button, deadzone, Filter, Jitter};

        if self.default_filters {
            let jitter_filter = Jitter::new();
            loop {
                let ev = self.next_event_priv()
                    .filter(&axis_dpad_to_button, self)
                    .filter(&jitter_filter, self)
                    .filter(&deadzone, self);

                // Skip all dropped events, there is no reason to return them
                match ev {
                    Some(ev) if ev.is_dropped() => (),
                    _ => break ev,
                }
            }
        } else {
            self.next_event_priv()
        }
    }

    /// Returns next pending event.
    fn next_event_priv(&mut self) -> Option<Event> {
        match self.inner.next_event() {
            Some(Event { id, mut event, time }) => {
                let gamepad = self.inner.gamepad_mut(id);
                match event {
                    EventType::ButtonPressed(_, nec) => {
                        event = EventType::ButtonPressed(gamepad.button_name(nec), nec);
                    }
                    EventType::ButtonReleased(_, nec) => {
                        event = EventType::ButtonReleased(gamepad.button_name(nec), nec);
                    }
                    EventType::AxisChanged(_, val, nec) => {
                        event = EventType::AxisChanged(gamepad.axis_name(nec), val, nec);
                    }
                    EventType::Connected => {
                        gamepad.status = Status::Connected;
                        let mapping = self.mappings
                            .get(gamepad.uuid())
                            .and_then(|s| {
                                Mapping::parse_sdl_mapping(
                                    s,
                                    gamepad.inner.buttons(),
                                    gamepad.inner.axes(),
                                ).ok()
                            })
                            .unwrap_or_default();
                        if !mapping.name().is_empty() {
                            gamepad.inner.set_name(mapping.name())
                        }
                        gamepad.mapping = mapping;

                        if gamepad.id == usize::max_value() {
                            gamepad.id = id;
                            gamepad.tx = self.tx.clone();

                            if let Some(device) = gamepad.inner.ff_device() {
                                let _ = self.tx.send(Message::Open { id, device });
                            }
                        }
                    }
                    EventType::Disconnected => {
                        gamepad.status = Status::Disconnected;
                        let _ = self.tx.send(Message::Close { id });
                    }
                    _ => (),
                };

                Some(Event { id, event, time })
            }
            None => None,
        }
    }

    /// Updates internal state according to `event`.
    pub fn update(&mut self, event: &Event) {
        use EventType::*;

        let counter = self.counter;

        let gamepad = match self.connected_gamepad_mut(event.id) {
            Some(g) => g,
            None => return,
        };

        match event.event {
            ButtonPressed(_, nec) => {
                gamepad
                    .state
                    .update_btn(nec, ButtonData::new(true, false, counter, event.time));
            }
            ButtonReleased(_, nec) => {
                gamepad
                    .state
                    .update_btn(nec, ButtonData::new(false, false, counter, event.time));
            }
            ButtonRepeated(_, nec) => {
                gamepad
                    .state
                    .update_btn(nec, ButtonData::new(true, true, counter, event.time));
            }
            AxisChanged(_, value, nec) => {
                gamepad
                    .state
                    .update_axis(nec, AxisData::new(value, counter, event.time));
            }
            _ => (),
        }
    }

    /// Increases internal counter by one. Counter data is stored with state and can be used to
    /// determine when last event happened. You probably want to use this function in your update
    /// loop after processing events.
    pub fn inc(&mut self) {
        // Counter is 62bit. See `ButtonData`.
        if self.counter == 0x3FFF_FFFF_FFFF_FFFF {
            self.counter = 0;
        } else {
            self.counter += 1;
        }
    }

    /// Returns counter. Counter data is stored with state and can be used to determine when last
    /// event happened.
    pub fn counter(&self) -> u64 {
        self.counter
    }

    /// Sets counter to 0.
    pub fn reset_counter(&mut self) {
        self.counter = 0;
    }

    fn create_ff_devices(&self) {
        for (id, gp) in self.gamepads()
            .filter(|&(_, g)| g.is_ff_supported())
            .map(|(id, g)| (id, g.inner.ff_device()))
        {
            if let Some(device) = gp {
                let _ = self.tx.send(Message::Open { id, device });
            }
        }
    }

    fn finish_gamepads_creation(&mut self) {
        let tx = self.tx.clone();
        for (id, gp) in self.gamepads_mut() {
            gp.id = id;
            gp.tx = tx.clone();
        }
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
        if gp.is_connected() {
            Some(gp)
        } else {
            None
        }
    }

    /// Returns a mutable reference to connected gamepad or `None`.
    pub fn connected_gamepad_mut(&mut self, id: usize) -> Option<&mut Gamepad> {
        let gp = self.inner.gamepad_mut(id);
        if gp.is_connected() {
            Some(gp)
        } else {
            None
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

/// Allow to create `Gilrs ` with customized behaviour.
pub struct GilrsBuilder {
    mappings: MappingDb,
    default_filters: bool,
}

impl GilrsBuilder {
    /// Create builder with default settings. Use `build()` to create `Gilrs`.
    pub fn new() -> Self {
        GilrsBuilder {
            mappings: MappingDb::without_env(),
            default_filters: true,
        }
    }

    /// If `true`, use [`axis_dpad_to_button`](ev/filter/fn.axis_dpad_to_button.html),
    /// [`Jitter`](ev/filter/struct.Jitter.html) and [`deadzone`](ev/filter/fn.deadzone.html)
    /// filters with default parameters. Defaults to `true`.
    pub fn with_default_filters(mut self, default_filters: bool) -> Self {
        self.default_filters = default_filters;

        self
    }

    /// Adds SDL mappings.
    pub fn add_mappings(mut self, mappings: &str) -> Self {
        self.mappings.insert(mappings);

        self
    }

    /// Creates `Gilrs`.
    pub fn build(mut self) -> Gilrs {
        self.mappings.add_env_mappings();

        let mut gilrs = Gilrs {
            inner: platform::Gilrs::new(),
            next_id: 0,
            tx: server::init(),
            counter: 0,
            mappings: self.mappings,
            default_filters: self.default_filters,
        };
        gilrs.finish_gamepads_creation();
        gilrs.create_ff_devices();

        gilrs
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
    mapping: Mapping,
    tx: Sender<Message>,
    id: usize,
}

impl Gamepad {
    fn new(gamepad: platform::Gamepad, status: Status) -> Self {
        Gamepad {
            inner: gamepad,
            state: GamepadState::new(),
            status,
            mapping: Mapping::new(),
            tx: ::std::sync::mpsc::channel().0,
            id: usize::max_value(),
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
    /// represented by axis returns true if value is not equal to 0.0. Panics if `btn` is `Unknown`.
    pub fn is_pressed(&self, btn: Button) -> bool {
        assert!(btn != Button::Unknown);

        self.button_code(btn)
            .or_else(|| btn.to_nec())
            .map(|nec| self.state.is_pressed(nec))
            .unwrap_or(false)
    }

    /// Examines cached gamepad state to check axis's value. If `axis` is represented by button on
    /// device it value is 0.0 if button is not pressed or 1.0 if is pressed. Panics if `axis` is
    /// `Unknown`.
    pub fn value(&self, axis: Axis) -> f32 {
        assert!(axis != Axis::Unknown);

        self.axis_code(axis)
            .map(|nec| self.state.value(nec))
            .unwrap_or(0.0)
    }

    /// Returns button state and when it changed.
    pub fn button_data(&self, btn: Button) -> Option<&ButtonData> {
        self.button_code(btn)
            .and_then(|nec| self.state.button_data(nec))
    }

    /// Returns axis state and when it changed.
    pub fn axis_data(&self, axis: Axis) -> Option<&AxisData> {
        self.axis_code(axis)
            .and_then(|nec| self.state.axis_data(nec))
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
    /// ```
    pub fn mapping_source(&self) -> MappingSource {
        if self.mapping.is_default() {
            // TODO: check if it's Driver or None
            MappingSource::Driver
        } else {
            MappingSource::SdlMappings
        }
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
        if !self.is_connected() {
            return Err(MappingError::NotConnected);
        }

        let name = match name.into() {
            Some(s) => s,
            None => self.inner.name(),
        };

        let (mapping, s) = Mapping::from_data(
            mapping,
            self.inner.buttons(),
            self.inner.axes(),
            name,
            self.uuid(),
        )?;
        self.mapping = mapping;

        Ok(s)
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
        if mapping.button(Button::C).is_some() || mapping.button(Button::Z).is_some()
            || mapping.axis(Axis::LeftZ).is_some()
            || mapping.axis(Axis::RightZ).is_some()
        {
            Err(MappingError::NotSdl2Compatible)
        } else {
            self.set_mapping(mapping, name)
        }
    }

    /// Returns true if force feedback is supported by device.
    pub fn is_ff_supported(&self) -> bool {
        self.inner.is_ff_supported()
    }

    /// Change gamepad position used by force feedback effects.
    pub fn set_listener_position<Vec3: Into<[f32; 3]>>(
        &self,
        position: Vec3,
    ) -> Result<(), FfError> {
        if !self.is_connected() {
            Err(FfError::Disconnected(self.id))
        } else if !self.is_ff_supported() {
            Err(FfError::FfNotSupported(self.id))
        } else {
            self.tx.send(Message::SetListenerPosition {
                id: self.id,
                position: position.into(),
            })?;
            Ok(())
        }
    }

    /// Returns `Button` mapped to `nec`.
    pub fn button_name(&self, nec: NativeEvCode) -> Button {
        self.mapping.map_button(nec)
    }

    /// Returns `Axis` mapped to `nec`.
    pub fn axis_name(&self, nec: NativeEvCode) -> Axis {
        self.mapping.map_axis(nec)
    }

    /// Returns `NativeEvCode` associated with `btn`.
    pub fn button_code(&self, btn: Button) -> Option<NativeEvCode> {
        self.mapping.map_rev_button(btn)
    }

    /// Returns `NativeEvCode` associated with `axis`.
    pub fn axis_code(&self, axis: Axis) -> Option<NativeEvCode> {
        self.mapping.map_rev_axis(axis)
    }

    /// Returns area in which axis events should be ignored.
    pub fn deadzone(&self, axis: NativeEvCode) -> f32 {
        self.inner.deadzone(axis)
    }

    /// Returns ID of gamepad.
    ///
    /// This function can return invalid ID if `Connected` event for this gamepad was not emitted.
    pub fn id(&self) -> usize {
        self.id
    }
}

// TODO: use pub(crate)
impl AsInner<platform::Gamepad> for Gamepad {
    fn as_inner(&self) -> &platform::Gamepad {
        &self.inner
    }

    fn as_inner_mut(&mut self) -> &mut platform::Gamepad {
        &mut self.inner
    }
}

// TODO: use pub(crate)
pub trait GamepadImplExt {
    fn from_inner_status(inner: platform::Gamepad, status: Status) -> Self;
}

// TODO: use pub(crate)
impl GamepadImplExt for Gamepad {
    fn from_inner_status(inner: platform::Gamepad, status: Status) -> Self {
        Self::new(inner, status)
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

/// Holds information about gamepad event.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Event {
    /// Id of gamepad.
    pub id: usize,
    /// Event's data.
    pub event: EventType,
    /// Time when event was emitted.
    pub time: SystemTime,
}

impl Event {
    /// Creates new event with current time.
    pub fn new(id: usize, event: EventType) -> Self {
        Event { id, event, time: SystemTime::now() }
    }

    /// Returns `Event` with `EventType::Dropped`.
    pub fn drop(mut self) -> Event {
        self.event = EventType::Dropped;

        self
    }

    /// Creates `Event` with `EventType::Dropped`.
    pub fn dropped() -> Event {
        Event {
            id: ::std::usize::MAX,
            event: EventType::Dropped,
            time: SystemTime::now(),
        }
    }

    /// Returns true if event is `Dropped` and should be ignored.
    pub fn is_dropped(&self) -> bool {
        self.event == EventType::Dropped
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Gamepad event.
pub enum EventType {
    /// Some button on gamepad has been pressed.
    ButtonPressed(Button, NativeEvCode),
    /// This event can ge generated by [`ev::Repeat`](ev/filter/struct.Repeat.html) event filter.
    ButtonRepeated(Button, NativeEvCode),
    /// Previously pressed button has been released.
    ButtonReleased(Button, NativeEvCode),
    /// Value of axis has changed. Value can be in range [-1.0, 1.0] for sticks and [0.0, 1.0] for
    /// triggers.
    AxisChanged(Axis, f32, NativeEvCode),
    /// Gamepad has been connected. If gamepad's UUID doesn't match one of disconnected gamepads,
    /// newly connected gamepad will get new ID. This event is also emitted when creating `Gilrs`
    /// for every gamepad that was already connected.
    Connected,
    /// Gamepad has been disconnected. Disconnected gamepad will not generate any new events.
    Disconnected,
    /// There was an `Event`, but it was dropped by one of filters. You should ignore it.
    Dropped,
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

    fn to_nec(self) -> Option<NativeEvCode> {
        use platform::native_ev_codes as necs;

        match self {
            Button::South => Some(necs::BTN_SOUTH),
            Button::East => Some(necs::BTN_EAST),
            Button::North => Some(necs::BTN_NORTH),
            Button::West => Some(necs::BTN_WEST),
            Button::C => Some(necs::BTN_C),
            Button::Z => Some(necs::BTN_Z),
            Button::LeftTrigger => Some(necs::BTN_LT),
            Button::LeftTrigger2 => Some(necs::BTN_LT2),
            Button::RightTrigger => Some(necs::BTN_RT),
            Button::RightTrigger2 => Some(necs::BTN_RT2),
            Button::Select => Some(necs::BTN_SELECT),
            Button::Start => Some(necs::BTN_START),
            Button::Mode => Some(necs::BTN_MODE),
            Button::LeftThumb => Some(necs::BTN_LTHUMB),
            Button::RightThumb => Some(necs::BTN_RTHUMB),
            Button::DPadUp => Some(necs::BTN_DPAD_UP),
            Button::DPadDown => Some(necs::BTN_DPAD_DOWN),
            Button::DPadLeft => Some(necs::BTN_DPAD_LEFT),
            Button::DPadRight => Some(necs::BTN_DPAD_RIGHT),
            _ => None,
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
    DPadX = AXIS_DPADX,
    DPadY = AXIS_DPADY,
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
