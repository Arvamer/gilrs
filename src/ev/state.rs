// Copyright 2017 Mateusz Sieczko and other GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use ev::EvCode;

use fnv::FnvHashMap;

use std::collections::hash_map;
use std::iter::Iterator;
use std::time::SystemTime;

/// Cached gamepad state.
#[derive(Clone, Debug)]
pub struct GamepadState {
    // Indexed by EvCode (nec)
    buttons: FnvHashMap<EvCode, ButtonData>,
    // Indexed by EvCode (nec)
    axes: FnvHashMap<EvCode, AxisData>,
}

impl GamepadState {
    pub(crate) fn new() -> Self {
        GamepadState {
            buttons: FnvHashMap::default(),
            axes: FnvHashMap::default(),
        }
    }

    /// Returns `true` if given button is pressed. Returns `false` if there is no information about
    /// `btn` or it is not pressed.
    pub fn is_pressed(&self, btn: &EvCode) -> bool {
        self.buttons
            .get(btn)
            .map(|s| s.is_pressed())
            .unwrap_or(false)
    }

    /// Returns value of axis or 0.0 when there is no information about axis.
    pub fn value(&self, axis: &EvCode) -> f32 {
        self.axes.get(axis).map(|s| s.value()).unwrap_or(0.0)
    }

    /// Iterate over buttons data.
    pub fn buttons(&self) -> ButtonDataIter {
        ButtonDataIter(self.buttons.iter())
    }

    /// Iterate over axes data.
    pub fn axes(&self) -> AxisDataIter {
        AxisDataIter(self.axes.iter())
    }

    /// Returns button state and when it changed.
    pub fn button_data(&self, btn: &EvCode) -> Option<&ButtonData> {
        self.buttons.get(btn)
    }

    /// Returns axis state and when it changed.
    pub fn axis_data(&self, axis: &EvCode) -> Option<&AxisData> {
        self.axes.get(axis)
    }

    pub(crate) fn update_btn(&mut self, btn: EvCode, data: ButtonData) {
        self.buttons.insert(btn, data);
    }

    pub(crate) fn update_axis(&mut self, axis: EvCode, data: AxisData) {
        self.axes.insert(axis, data);
    }
}

/// Iterator over `ButtonData`.
pub struct ButtonDataIter<'a>(hash_map::Iter<'a, EvCode, ButtonData>);

/// Iterator over `AxisData`.
pub struct AxisDataIter<'a>(hash_map::Iter<'a, EvCode, AxisData>);

impl<'a> Iterator for ButtonDataIter<'a> {
    type Item = (EvCode, &'a ButtonData);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(k, v)| (*k, v))
    }
}

impl<'a> Iterator for AxisDataIter<'a> {
    type Item = (EvCode, &'a AxisData);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(k, v)| (*k, v))
    }
}

/// Information about button stored in `State`.
#[derive(Clone, Copy, Debug)]
pub struct ButtonData {
    last_event_ts: SystemTime,
    state_and_counter: u64,
    // 2b of state (is pressed, is repeating), 62b of counter
}

impl ButtonData {
    pub(crate) fn new(pressed: bool, repeating: bool, counter: u64, time: SystemTime) -> Self {
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

/// Information about axis stored in `State`.
#[derive(Clone, Copy, Debug)]
pub struct AxisData {
    last_event_ts: SystemTime,
    last_event_c: u64,
    value: f32,
}

impl AxisData {
    pub(crate) fn new(value: f32, counter: u64, time: SystemTime) -> Self {
        AxisData {
            last_event_ts: time,
            last_event_c: counter,
            value,
        }
    }
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
            let btn = ButtonData::new(pressed, repeating, counter, SystemTime::now());
            assert_eq!(btn.is_pressed(), pressed);
            assert_eq!(btn.is_repeating(), repeating);
            assert_eq!(btn.counter(), counter);
        }
    }
}
