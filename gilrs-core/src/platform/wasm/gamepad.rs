// Copyright 2016-2018 Mateusz Sieczko and other GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use std::collections::VecDeque;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::time::Duration;

use js_sys::RegExp;
use uuid::Uuid;
use wasm_bindgen::JsCast;
use web_sys::{DomException, Gamepad as WebGamepad, GamepadButton, GamepadMappingType};

use super::FfDevice;
use crate::platform::native_ev_codes::{BTN_LT2, BTN_RT2};
use crate::{AxisInfo, Event, EventType, PlatformError, PowerInfo};
#[cfg(feature = "serde-serialize")]
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct Gilrs {
    event_cache: VecDeque<Event>,
    gamepads: Vec<Gamepad>,
    new_web_gamepads: Vec<WebGamepad>,
    next_event_error_logged: bool,
}

impl Gilrs {
    pub(crate) fn new() -> Result<Self, PlatformError> {
        let window =
            web_sys::window().ok_or_else(|| PlatformError::Other(Box::new(Error::NoWindow)))?;
        if !window.is_secure_context() {
            warn!("Context is not secure, gamepad API may not be available.")
        }

        Ok({
            Gilrs {
                event_cache: VecDeque::new(),
                gamepads: Vec::new(),
                new_web_gamepads: Vec::new(),
                next_event_error_logged: false,
            }
        })
    }

    pub(crate) fn next_event(&mut self) -> Option<Event> {
        // Don't duplicate the work of checking the diff between the old and new gamepads if
        // there are still events to return
        if !self.event_cache.is_empty() {
            return self.event_cache.pop_front();
        }

        let gamepads = match web_sys::window()
            .expect("no window")
            .navigator()
            .get_gamepads()
        {
            Ok(x) => {
                self.next_event_error_logged = false;
                x
            }
            Err(js) => {
                if !self.next_event_error_logged {
                    self.next_event_error_logged = true;

                    let exception: DomException = match js.dyn_into() {
                        Ok(x) => x,
                        Err(e) => {
                            error!("getGamepads() failed with unknown error: {:?}", e);
                            return None;
                        }
                    };
                    error!("getGamepads(): {}", exception.message());
                }

                return None;
            }
        };

        // Gather all non-null gamepads
        for maybe_js_gamepad in gamepads {
            if !maybe_js_gamepad.is_null() {
                self.new_web_gamepads
                    .push(WebGamepad::from(maybe_js_gamepad));
            }
        }

        // Update existing gamepads
        for (id, gamepad) in self.gamepads.iter_mut().enumerate() {
            let maybe_js_gamepad_index = self
                .new_web_gamepads
                .iter()
                .position(|x| gamepad.gamepad.index() == x.index());
            if let Some(js_gamepad_index) = maybe_js_gamepad_index {
                gamepad.gamepad = self.new_web_gamepads.swap_remove(js_gamepad_index);

                if !gamepad.connected {
                    self.event_cache
                        .push_back(Event::new(id, EventType::Connected));
                    gamepad.connected = true;
                }

                let buttons = gamepad.gamepad.buttons();
                for btn_index in 0..gamepad
                    .mapping
                    .buttons()
                    .len()
                    .min(buttons.length() as usize)
                {
                    let (old_pressed, old_value) = gamepad.mapping.buttons()[btn_index];

                    let ev_code = crate::EvCode(gamepad.button_code(btn_index));
                    let button_object = GamepadButton::from(buttons.get(btn_index as u32));

                    let new_pressed = button_object.pressed();
                    let new_value = button_object.value();

                    if [BTN_LT2, BTN_RT2].contains(&ev_code.0) && old_value != new_value {
                        // Treat left and right triggers as axes so we get non-binary values.
                        // Button Pressed/Changed events are generated from the axis changed
                        // events later.
                        let value = (new_value * i32::MAX as f64) as i32;
                        self.event_cache
                            .push_back(Event::new(id, EventType::AxisValueChanged(value, ev_code)));
                    } else {
                        match (old_pressed, new_pressed) {
                            (false, true) => self
                                .event_cache
                                .push_back(Event::new(id, EventType::ButtonPressed(ev_code))),
                            (true, false) => self
                                .event_cache
                                .push_back(Event::new(id, EventType::ButtonReleased(ev_code))),
                            _ => (),
                        }
                    }

                    gamepad.mapping.buttons_mut()[btn_index] = (new_pressed, new_value);
                }

                let axes = gamepad.gamepad.axes();
                for axis_index in 0..gamepad.mapping.axes().len().min(axes.length() as usize) {
                    let old_value = gamepad.mapping.axes()[axis_index];
                    let new_value = axes
                        .get(axis_index as u32)
                        .as_f64()
                        .expect("axes() should be an array of f64");
                    if old_value != new_value {
                        let ev_code = crate::EvCode(gamepad.axis_code(axis_index));
                        let value = (new_value * i32::MAX as f64) as i32;
                        self.event_cache
                            .push_back(Event::new(id, EventType::AxisValueChanged(value, ev_code)));
                    }

                    gamepad.mapping.axes_mut()[axis_index] = new_value;
                }
            } else {
                // Create a disconnect event
                if gamepad.connected {
                    self.event_cache
                        .push_back(Event::new(id, EventType::Disconnected));
                    gamepad.connected = false;
                }
            }
        }

        // Add new gamepads
        for js_gamepad in self.new_web_gamepads.drain(..) {
            let id = self.gamepads.len();
            self.gamepads.push(Gamepad::new(js_gamepad));

            // Create a connected event
            let event = Event::new(id, EventType::Connected);
            self.event_cache.push_back(event);
        }

        self.event_cache.pop_front()
    }

    pub(crate) fn next_event_blocking(&mut self, _timeout: Option<Duration>) -> Option<Event> {
        unimplemented!("next_event_blocking is not supported on web. Use next_event.")
    }

    pub fn gamepad(&self, id: usize) -> Option<&Gamepad> {
        self.gamepads.get(id)
    }

    pub fn last_gamepad_hint(&self) -> usize {
        self.gamepads.len()
    }
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
enum Mapping {
    Standard {
        buttons: [(bool, f64); 17],
        axes: [f64; 4],
    },
    NoMapping {
        buttons: Vec<(bool, f64)>,
        axes: Vec<f64>,
    },
}

impl Mapping {
    fn buttons(&self) -> &[(bool, f64)] {
        match self {
            Mapping::Standard { buttons, .. } => buttons,
            Mapping::NoMapping { buttons, .. } => buttons,
        }
    }

    fn buttons_mut(&mut self) -> &mut [(bool, f64)] {
        match self {
            Mapping::Standard { buttons, .. } => &mut *buttons,
            Mapping::NoMapping { buttons, .. } => &mut *buttons,
        }
    }

    fn axes(&self) -> &[f64] {
        match self {
            Mapping::Standard { axes, .. } => axes,
            Mapping::NoMapping { axes, .. } => axes,
        }
    }

    fn axes_mut(&mut self) -> &mut [f64] {
        match self {
            Mapping::Standard { axes, .. } => &mut *axes,
            Mapping::NoMapping { axes, .. } => &mut *axes,
        }
    }
}

#[derive(Debug)]
pub struct Gamepad {
    uuid: Uuid,
    gamepad: WebGamepad,
    name: String,
    vendor: Option<u16>,
    product: Option<u16>,
    mapping: Mapping,
    connected: bool,
}

impl Gamepad {
    fn new(gamepad: WebGamepad) -> Gamepad {
        let name = gamepad.id();

        // This regular expression extracts the vendor and product ID from the gamepad "id".
        // Firefox:
        //  054c-05c4-Sony Computer Entertainment Wireless Controller
        // Chrome:
        //  Sony Computer Entertainment Wireless Controller (STANDARD GAMEPAD Vendor: 054c Product: 05c4)
        let regexp = RegExp::new(
            r"(?:^([a-f0-9]{4})-([a-f0-9]{4})-)|(?:Vendor: ([a-f0-9]{4}) Product: ([a-f0-9]{4})\)$)",
            "",
        );
        let (vendor, product) = if let Some(matches) = regexp.exec(&name) {
            let parse_hex = |index| {
                matches
                    .get(index)
                    .as_string()
                    .and_then(|id| u16::from_str_radix(&id, 16).ok())
            };
            (
                parse_hex(1).or_else(|| parse_hex(3)),
                parse_hex(2).or_else(|| parse_hex(4)),
            )
        } else {
            (None, None)
        };

        let buttons = gamepad.buttons();
        let button_iter = {
            {
                buttons.iter().map(GamepadButton::from)
            }
        };

        let axes = gamepad.axes();
        let axis_iter = {
            {
                axes.iter()
                    .map(|val| val.as_f64().expect("axes() should be an array of f64"))
            }
        };

        let mapping = match gamepad.mapping() {
            GamepadMappingType::Standard => {
                let mut buttons = [(false, 0.0); 17];
                let mut axes = [0.0; 4];

                for (index, button) in button_iter.enumerate().take(buttons.len()) {
                    buttons[index] = (button.pressed(), button.value());
                }

                for (index, axis) in axis_iter.enumerate().take(axes.len()) {
                    axes[index] = axis;
                }

                Mapping::Standard { buttons, axes }
            }
            _ => {
                let buttons = button_iter
                    .map(|button| (button.pressed(), button.value()))
                    .collect();
                let axes = axis_iter.collect();
                Mapping::NoMapping { buttons, axes }
            }
        };

        Gamepad {
            uuid: Uuid::nil(),
            gamepad,
            name,
            vendor,
            product,
            mapping,
            connected: true,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    pub fn vendor_id(&self) -> Option<u16> {
        self.vendor
    }

    pub fn product_id(&self) -> Option<u16> {
        self.product
    }

    pub fn is_connected(&self) -> bool {
        self.gamepad.connected()
    }

    pub fn power_info(&self) -> PowerInfo {
        PowerInfo::Unknown
    }

    pub fn is_ff_supported(&self) -> bool {
        false
    }

    pub fn ff_device(&self) -> Option<FfDevice> {
        None
    }

    pub fn buttons(&self) -> &[EvCode] {
        &native_ev_codes::BUTTONS
    }

    pub fn axes(&self) -> &[EvCode] {
        &native_ev_codes::AXES
    }

    fn button_code(&self, index: usize) -> EvCode {
        self.buttons()
            .get(index)
            .copied()
            .unwrap_or(EvCode(index as u8 + 31))
    }

    fn axis_code(&self, index: usize) -> EvCode {
        self.axes()
            .get(index)
            .copied()
            .unwrap_or_else(|| EvCode((index + self.mapping.buttons().len()) as u8 + 31))
    }

    pub(crate) fn axis_info(&self, _nec: EvCode) -> Option<&AxisInfo> {
        if self.buttons().contains(&_nec) {
            return Some(&AxisInfo {
                min: 0,
                max: i32::MAX,
                deadzone: None,
            });
        }
        Some(&AxisInfo {
            min: i32::MIN,
            max: i32::MAX,
            deadzone: None,
        })
    }
}

#[cfg_attr(feature = "serde-serialize", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct EvCode(u8);

impl EvCode {
    pub fn into_u32(self) -> u32 {
        self.0 as u32
    }
}

impl Display for EvCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        self.0.fmt(f)
    }
}

#[derive(Debug, Copy, Clone)]
enum Error {
    NoWindow,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match *self {
            Error::NoWindow => f.write_str("window is not available"),
        }
    }
}

impl std::error::Error for Error {}

pub mod native_ev_codes {
    use super::EvCode;

    pub const AXIS_LSTICKX: EvCode = EvCode(0);
    pub const AXIS_LSTICKY: EvCode = EvCode(1);
    pub const AXIS_LEFTZ: EvCode = EvCode(2);
    pub const AXIS_RSTICKX: EvCode = EvCode(3);
    pub const AXIS_RSTICKY: EvCode = EvCode(4);
    pub const AXIS_RIGHTZ: EvCode = EvCode(5);
    pub const AXIS_DPADX: EvCode = EvCode(6);
    pub const AXIS_DPADY: EvCode = EvCode(7);
    pub const AXIS_RT: EvCode = EvCode(8);
    pub const AXIS_LT: EvCode = EvCode(9);
    pub const AXIS_RT2: EvCode = EvCode(10);
    pub const AXIS_LT2: EvCode = EvCode(11);

    pub const BTN_SOUTH: EvCode = EvCode(12);
    pub const BTN_EAST: EvCode = EvCode(13);
    pub const BTN_C: EvCode = EvCode(14);
    pub const BTN_NORTH: EvCode = EvCode(15);
    pub const BTN_WEST: EvCode = EvCode(16);
    pub const BTN_Z: EvCode = EvCode(17);
    pub const BTN_LT: EvCode = EvCode(18);
    pub const BTN_RT: EvCode = EvCode(19);
    pub const BTN_LT2: EvCode = EvCode(20);
    pub const BTN_RT2: EvCode = EvCode(21);
    pub const BTN_SELECT: EvCode = EvCode(22);
    pub const BTN_START: EvCode = EvCode(23);
    pub const BTN_MODE: EvCode = EvCode(24);
    pub const BTN_LTHUMB: EvCode = EvCode(25);
    pub const BTN_RTHUMB: EvCode = EvCode(26);

    pub const BTN_DPAD_UP: EvCode = EvCode(27);
    pub const BTN_DPAD_DOWN: EvCode = EvCode(28);
    pub const BTN_DPAD_LEFT: EvCode = EvCode(29);
    pub const BTN_DPAD_RIGHT: EvCode = EvCode(30);

    pub(super) static BUTTONS: [EvCode; 17] = [
        BTN_SOUTH,
        BTN_EAST,
        BTN_WEST,
        BTN_NORTH,
        BTN_LT,
        BTN_RT,
        BTN_LT2,
        BTN_RT2,
        BTN_SELECT,
        BTN_START,
        BTN_LTHUMB,
        BTN_RTHUMB,
        BTN_DPAD_UP,
        BTN_DPAD_DOWN,
        BTN_DPAD_LEFT,
        BTN_DPAD_RIGHT,
        BTN_MODE,
    ];

    pub(super) static AXES: [EvCode; 4] = [AXIS_LSTICKX, AXIS_LSTICKY, AXIS_RSTICKX, AXIS_RSTICKY];
}
