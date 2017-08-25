// Copyright 2017 Mateusz Sieczko and other GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

//! Gamepad state and other event related functionality.

use gamepad::{Axis, Button, Event, NativeEvCode};

use vec_map::VecMap;

use std::time::SystemTime;

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
    counter: u64, // max 63bits
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
                    ButtonData::new(true, self.counter, event.time),
                );
                if btn != Button::Unknown {
                    gamepad.btn_to_nec.insert(btn as usize, nec);
                }
            }
            ButtonReleased(_, nec) => {
                gamepad.buttons.insert(
                    nec as usize,
                    ButtonData::new(false, self.counter, event.time),
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
        // Counter is 63bit. See `ButtonData`.
        if self.counter == 0x7FFF_FFFF_FFFF_FFFF {
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
}

struct GamepadState {
    // Indexed by NativeEvCode (nec)
    buttons: VecMap<ButtonData>,
    // Indexed by NativeEvCode (nec)
    axes: VecMap<AxisData>,
    // Mappings, will be dynamically created while we are processing new events.
    btn_to_nec: VecMap<u16>,
    axis_to_nec: VecMap<u16>,
}

impl GamepadState {
    fn new() -> Self {
        GamepadState {
            buttons: VecMap::new(),
            axes: VecMap::new(),
            btn_to_nec: VecMap::new(),
            axis_to_nec: VecMap::new(),
        }
    }
}

/// Information about button stored in `State`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ButtonData {
    last_event_ts: SystemTime,
    state_and_counter: u64,
    // 1b of state, 63b of counter
}

impl ButtonData {
    fn new(pressed: bool, counter: u64, time: SystemTime) -> Self {
        ButtonData {
            last_event_ts: time,
            state_and_counter: ((pressed as u64) << 63) | counter,
        }
    }

    /// Returns `true` if button is pressed.
    pub fn is_pressed(&self) -> bool {
        self.state_and_counter >> 63 == 1
    }

    /// Returns value of counter when button state last changed.
    pub fn counter(&self) -> u64 {
        self.state_and_counter & 0x7FFF_FFFF_FFFF_FFFF
    }

    /// Returns when button state last changed.
    pub fn timestamp(&self) -> SystemTime {
        self.last_event_ts
    }

    #[allow(dead_code)]
    fn set_state(&mut self, pressed: bool) {
        self.state_and_counter = ((self.state_and_counter << 1) + pressed as u64).rotate_right(1);
    }

    #[allow(dead_code)]
    fn set_counter(&mut self, counter: u64) {
        self.state_and_counter = self.state_and_counter & 0x8000_0000_0000_0000 | counter;
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
            let counter = xorshift() & 0x7FFF_FFFF_FFFF_FFFF;
            let pressed = xorshift() % 2 == 1;
            let mut btn = ButtonData::new(pressed, counter, SystemTime::now());
            assert_eq!(btn.is_pressed(), pressed);
            assert_eq!(btn.counter(), counter);

            let counter = xorshift() & 0x7FFF_FFFF_FFFF_FFFF;
            let pressed = xorshift() % 2 == 1;
            btn.set_counter(counter);
            btn.set_state(pressed);
            assert_eq!(btn.is_pressed(), pressed);
            assert_eq!(btn.counter(), counter);
        }
    }
}
