// Copyright 2017 Mateusz Sieczko and other GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

//! Gamepad state and other event related functionality.

use gamepad::{Axis, Button, Event, NativeEvCode};

use vec_map::{self, VecMap};

use std::time::SystemTime;
use std::iter::Iterator;

pub mod filter;

/// Stores state of gamepads.
///
/// To update stored state use `update` function. This struct also store when state changed.
///
/// # Counter
///
/// `State` has additional functionality, referred here as *counter*. The idea behind it is simple,
/// each time you end iteration of update loop, you call `State::inc()` which will increase
/// internal counter by one. When state of one if elements changes, value of counter is saved. When
/// checking state of one of elements you can tell exactly when this event happened. Timestamps are
/// not good solution here because they can tell you when *system* observed event, not when you
/// processed it. On the other hand, they are good when you want to implement key repeat or software
/// debouncing.
///
/// ```
/// use gilrs::{Gilrs, Button};
/// use gilrs::ev::State;
///
/// let mut gilrs = Gilrs::new();
/// let mut gamepad_state = State::new();
///
/// loop {
///     for ev in gilrs.poll_events() {
///         gamepad_state.update(&ev);
///         // Do other things with event
///     }
///
///     if gamepad_state.is_pressed(/* id: */ 0, Button::DPadLeft) {
///         // go left
///     }
///
///     match gamepad_state.button_data(0, Button::South) {
///         Some(d) if d.is_pressed() && d.counter() == gamepad_state.counter() => {
///             // jump
///         }
///         _ => ()
///     }
///
///     gamepad_state.inc();
/// #   break;
/// }
///
pub struct State {
    gamepads: VecMap<GamepadState>,
    counter: u64, // max 62bits
}

impl State {
    /// Creates new `State`.
    pub fn new() -> Self {
        State { gamepads: VecMap::new(), counter: 0 }
    }

    /// Updates state according to `event`.
    pub fn update(&mut self, event: &Event) {
        use gamepad::EventType::*;

        let gamepad = self.gamepads.entry(event.id).or_insert(GamepadState::new());

        match event.event {
            ButtonPressed(btn, nec) => {
                gamepad.buttons.insert(
                    nec as usize,
                    ButtonData::new(true, false, self.counter, event.time),
                );
                if btn != Button::Unknown {
                    gamepad.btn_to_nec.insert(btn as usize, nec);
                    gamepad.nec_to_btn.insert(nec as usize, btn);
                }
            }
            ButtonReleased(_, nec) => {
                gamepad.buttons.insert(
                    nec as usize,
                    ButtonData::new(false, false, self.counter, event.time),
                );
            }
            ButtonRepeated(_, nec) => {
                gamepad.buttons.insert(
                    nec as usize,
                    ButtonData::new(true, true, self.counter, event.time),
                );
            }
            AxisChanged(axis, value, nec) => {
                gamepad.axes.insert(
                    nec as usize,
                    AxisData {
                        last_event_ts: event.time,
                        last_event_c: self.counter,
                        value,
                    },
                );
                if axis != Axis::Unknown {
                    gamepad.axis_to_nec.insert(axis as usize, nec);
                    gamepad.nec_to_axis.insert(nec as usize, axis);
                }
            }
            _ => (),
        }
    }

    /// Returns `true` if given button is pressed. Returns `false` if there is no information about
    /// `btn` or it is not pressed.
    pub fn is_pressed(&self, id: usize, btn: Button) -> bool {
        self.button_data(id, btn)
            .map(|data| data.is_pressed())
            .unwrap_or(false)
    }

    /// Returns `true` if given button is pressed. Returns `false` if there is no information about
    /// `btn` or it is not pressed.
    pub fn is_pressed_nec(&self, id: usize, btn: NativeEvCode) -> bool {
        self.button_data_nec(id, btn)
            .map(|data| data.is_pressed())
            .unwrap_or(false)
    }

    /// Returns button state and when it changed.
    pub fn button_data(&self, id: usize, btn: Button) -> Option<&ButtonData> {
        assert!(btn != Button::Unknown);

        if let Some(state) = self.gamepads.get(id) {
            state.btn_to_nec.get(btn as usize).cloned().and_then(
                |nec| {
                    state.buttons.get(nec as usize)
                },
            )
        } else {
            None
        }
    }

    /// Returns value of axis or 0.0 when there is no information about axis.
    pub fn value(&self, id: usize, axis: Axis) -> f32 {
        self.axis_data(id, axis)
            .map(|data| data.value())
            .unwrap_or(0.0)
    }

    /// Returns `true` if given button is pressed. Returns `false` if there is no information about
    /// `btn` or it is not pressed.
    pub fn value_nec(&self, id: usize, axis: NativeEvCode) -> f32 {
        self.axis_data_nec(id, axis)
            .map(|data| data.value())
            .unwrap_or(0.0)
    }

    /// Returns button state and when it changed.
    pub fn button_data_nec(&self, id: usize, btn: NativeEvCode) -> Option<&ButtonData> {
        self.gamepads.get(id).and_then(|gamepad| {
            gamepad.buttons.get(btn as usize)
        })
    }

    /// Returns axis state and when it changed.
    pub fn axis_data(&self, id: usize, axis: Axis) -> Option<&AxisData> {
        assert!(axis != Axis::Unknown);

        if let Some(state) = self.gamepads.get(id) {
            state.axis_to_nec.get(axis as usize).cloned().and_then(
                |nec| {
                    state.axes.get(nec as usize)
                },
            )
        } else {
            None
        }
    }

    /// Returns axis state and when it changed.
    pub fn axis_data_nec(&self, id: usize, axis: NativeEvCode) -> Option<&AxisData> {
        self.gamepads.get(id).and_then(|gamepad| {
            gamepad.axes.get(axis as usize)
        })
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

    pub fn gamepads(&self) -> GamepadStateIter {
        GamepadStateIter(self.gamepads.iter())
    }
}

pub struct GamepadState {
    // Indexed by NativeEvCode (nec)
    buttons: VecMap<ButtonData>,
    // Indexed by NativeEvCode (nec)
    axes: VecMap<AxisData>,
    // Mappings, will be dynamically created while we are processing new events.
    btn_to_nec: VecMap<u16>,
    axis_to_nec: VecMap<u16>,
    nec_to_btn: VecMap<Button>,
    nec_to_axis: VecMap<Axis>,
}

impl GamepadState {
    fn new() -> Self {
        GamepadState {
            buttons: VecMap::new(),
            axes: VecMap::new(),
            btn_to_nec: VecMap::new(),
            axis_to_nec: VecMap::new(),
            nec_to_btn: VecMap::new(),
            nec_to_axis: VecMap::new(),
        }
    }

    /// Iterate over buttons data.
    pub fn buttons(&self) -> ButtonDataIter {
        ButtonDataIter(self.buttons.iter())
    }

    /// Iterate over axes data.
    pub fn axes(&self) -> AxisDataIter {
        AxisDataIter(self.axes.iter())
    }

    /// Maps `NativeEvCode` to `Button`. Return `Button::Unknown` if no mapping found.
    pub fn nec_to_btn(&self, nec: NativeEvCode) -> Button {
        self.nec_to_btn.get(nec as usize).cloned().unwrap_or(Button::Unknown)
    }

    /// Maps `NativeEvCode` to `Axis`. Return `Axis::Unknown` if no mapping found.
    pub fn nec_to_axis(&self, nec: NativeEvCode) -> Axis {
        self.nec_to_axis.get(nec as usize).cloned().unwrap_or(Axis::Unknown)
    }
}

pub struct GamepadStateIter<'a>(vec_map::Iter<'a, GamepadState>);
pub struct ButtonDataIter<'a>(vec_map::Iter<'a, ButtonData>);
pub struct AxisDataIter<'a>(vec_map::Iter<'a, AxisData>);

impl<'a> Iterator for GamepadStateIter<'a> {
    type Item = (usize, &'a GamepadState);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<'a> Iterator for ButtonDataIter<'a> {
    type Item = (usize, &'a ButtonData);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl<'a> Iterator for AxisDataIter<'a> {
    type Item = (usize, &'a AxisData);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

/// Information about button stored in `State`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ButtonData {
    last_event_ts: SystemTime,
    state_and_counter: u64,
    // 2b of state (is pressed, is repeating), 62b of counter
}

impl ButtonData {
    fn new(pressed: bool, repeating: bool, counter: u64, time: SystemTime) -> Self {
        debug_assert!(counter <= 0x3FFF_FFFF_FFFF_FFFF);

        let state = ((pressed as u64) << 63) | ((repeating as u64) << 62);
        ButtonData {
            last_event_ts: time,
            state_and_counter: state | counter,
        }
    }

    /// Returns `true` if button is pressed.
    pub fn is_pressed(&self) -> bool {
        self.state_and_counter >> 63 == 1
    }

    /// Returns `true` if button is repeating.
    pub fn is_repeating(&self) -> bool {
        self.state_and_counter & 0x4000_0000_0000_0000 != 0
    }

    /// Returns value of counter when button state last changed.
    pub fn counter(&self) -> u64 {
        self.state_and_counter & 0x3FFF_FFFF_FFFF_FFFF
    }

    /// Returns when button state last changed.
    pub fn timestamp(&self) -> SystemTime {
        self.last_event_ts
    }
}

/// Information axis button stored in `State`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxisData {
    last_event_ts: SystemTime,
    last_event_c: u64,
    value: f32,
}

impl AxisData {
    /// Returns value of axis.
    pub fn value(&self) -> f32 {
        self.value
    }

    /// Returns value of counter when axis value last changed.
    pub fn counter(&self) -> u64 {
        self.last_event_c
    }

    /// Returns when axis value last changed.
    pub fn timestamp(&self) -> SystemTime {
        self.last_event_ts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    #[test]
    fn button_data() {
        // Who needs rand crate
        let mut state = 1234567890123456789u64;
        let mut xorshift = || {
            let mut x = state;
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            state = x;

            x.wrapping_mul(0x2545F4914F6CDD1D)
        };

        for _ in 0..(1024 * 1024 * 16) {
            let counter = xorshift() & 0x3FFF_FFFF_FFFF_FFFF;
            let pressed = xorshift() % 2 == 1;
            let repeating = xorshift() % 2 == 1;
            let mut btn = ButtonData::new(pressed, repeating, counter, SystemTime::now());
            assert_eq!(btn.is_pressed(), pressed);
            assert_eq!(btn.is_repeating(), repeating);
            assert_eq!(btn.counter(), counter);
        }
    }
}
