// Copyright 2016 GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
#![allow(unused_variables)]

use gamepad::{self, Event, Status, Axis, Button, GamepadImplExt};
use uuid::Uuid;
use std::thread;
use std::mem;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;
use std::u32::MAX as U32_MAX;
use std::i16::MAX as I16_MAX;
use std::u8::MAX as U8_MAX;
use winapi::winerror::{ERROR_SUCCESS, ERROR_DEVICE_NOT_CONNECTED};
use winapi::xinput::{XINPUT_STATE as XState, XINPUT_GAMEPAD_DPAD_UP, XINPUT_GAMEPAD_DPAD_DOWN,
                     XINPUT_GAMEPAD_DPAD_LEFT, XINPUT_GAMEPAD_DPAD_RIGHT, XINPUT_GAMEPAD_START,
                     XINPUT_GAMEPAD_BACK, XINPUT_GAMEPAD_LEFT_THUMB, XINPUT_GAMEPAD_RIGHT_THUMB,
                     XINPUT_GAMEPAD_LEFT_SHOULDER, XINPUT_GAMEPAD_RIGHT_SHOULDER,
                     XINPUT_GAMEPAD_A, XINPUT_GAMEPAD_B, XINPUT_GAMEPAD_X, XINPUT_GAMEPAD_Y,
                     XINPUT_GAMEPAD as XGamepad};

use xinput;

const EVENT_THREAD_SLEEP_TIME: u64 = 10;
const ITERATIONS_TO_CHECK_IF_CONNECTED: u64 = 100;

#[derive(Debug)]
pub struct Gilrs {
    gamepads: [gamepad::Gamepad; 4],
    rx: Receiver<(usize, Event)>,
    not_observed: gamepad::Gamepad,
}

impl Gilrs {
    pub fn new() -> Self {
        let gamepads = [gamepad_new(0), gamepad_new(1), gamepad_new(2), gamepad_new(3)];
        let connected = [gamepads[0].is_connected(),
                         gamepads[1].is_connected(),
                         gamepads[2].is_connected(),
                         gamepads[3].is_connected()];
        unsafe { xinput::XInputEnable(1) };
        let (tx, rx) = mpsc::channel::<(usize, Event)>();
        Self::spawn_thread(tx, connected);
        Gilrs {
            gamepads: gamepads,
            rx: rx,
            not_observed: gamepad::Gamepad::from_inner_status(Gamepad::none(), Status::NotObserved),
        }
    }

    pub fn poll_events(&mut self) -> EventIterator {
        EventIterator(self)
    }

    pub fn gamepad(&self, id: usize) -> &gamepad::Gamepad {
        self.gamepads.get(id).unwrap_or(&self.not_observed)
    }

    pub fn gamepad_mut(&mut self, id: usize) -> &mut gamepad::Gamepad {
        self.gamepads.get_mut(id).unwrap_or(&mut self.not_observed)
    }

    fn spawn_thread(tx: Sender<(usize, Event)>, connected: [bool; 4]) {
        thread::spawn(move || {
            unsafe {
                let mut prev_state = mem::zeroed::<XState>();
                let mut state = mem::zeroed::<XState>();
                let mut connected = connected;
                let mut counter = 0;

                loop {
                    for id in 0..4 {
                        if *connected.get_unchecked(id) ||
                           counter % ITERATIONS_TO_CHECK_IF_CONNECTED == 0 {
                            let val = xinput::XInputGetState(id as u32, &mut state as &mut _);
                            if val == ERROR_SUCCESS {
                                if !connected.get_unchecked(id) {
                                    *connected.get_unchecked_mut(id) = true;
                                    let _ = tx.send((id, Event::Connected));
                                }

                                if state.dwPacketNumber != prev_state.dwPacketNumber {
                                    Self::compare_state(id,
                                                        &state.Gamepad,
                                                        &prev_state.Gamepad,
                                                        &tx);
                                    prev_state = state;
                                }
                            } else if val == ERROR_DEVICE_NOT_CONNECTED &&
                                      *connected.get_unchecked(id) {
                                *connected.get_unchecked_mut(id) = false;
                                let _ = tx.send((id, Event::Disconnected));
                            }
                        }
                    }

                    counter = counter.wrapping_add(1);
                    thread::sleep(Duration::from_millis(EVENT_THREAD_SLEEP_TIME));
                }
            }
        });
    }

    fn compare_state(id: usize, g: &XGamepad, pg: &XGamepad, tx: &Sender<(usize, Event)>) {
        if g.bLeftTrigger != pg.bLeftTrigger {
            let _ = tx.send((id,
                             Event::AxisChanged(Axis::LeftTrigger2,
                                                g.bLeftTrigger as f32 / U8_MAX as f32)));
        }
        if g.bRightTrigger != pg.bRightTrigger {
            let _ = tx.send((id,
                             Event::AxisChanged(Axis::RightTrigger2,
                                                g.bRightTrigger as f32 / U8_MAX as f32)));
        }
        if g.sThumbLX != pg.sThumbLX {
            let _ = tx.send((id,
                             Event::AxisChanged(Axis::LeftStickX,
                                                g.sThumbLX as f32 / I16_MAX as f32)));
        }
        if g.sThumbLY != pg.sThumbLY {
            let _ = tx.send((id,
                             Event::AxisChanged(Axis::LeftStickY,
                                                g.sThumbLY as f32 / I16_MAX as f32)));
        }
        if g.sThumbRX != pg.sThumbRX {
            let _ = tx.send((id,
                             Event::AxisChanged(Axis::RightStickX,
                                                g.sThumbRX as f32 / I16_MAX as f32)));
        }
        if g.sThumbRY != pg.sThumbRY {
            let _ = tx.send((id,
                             Event::AxisChanged(Axis::RightStickY,
                                                g.sThumbRY as f32 / I16_MAX as f32)));
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_DPAD_UP) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_DPAD_UP != 0 {
                true => tx.send((id, Event::ButtonPressed(Button::DPadUp))),
                false => tx.send((id, Event::ButtonReleased(Button::DPadUp))),
            };
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_DPAD_DOWN) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_DPAD_DOWN != 0 {
                true => tx.send((id, Event::ButtonPressed(Button::DPadDown))),
                false => tx.send((id, Event::ButtonReleased(Button::DPadDown))),
            };
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_DPAD_LEFT) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_DPAD_LEFT != 0 {
                true => tx.send((id, Event::ButtonPressed(Button::DPadLeft))),
                false => tx.send((id, Event::ButtonReleased(Button::DPadLeft))),
            };
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_DPAD_RIGHT) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_DPAD_RIGHT != 0 {
                true => tx.send((id, Event::ButtonPressed(Button::DPadRight))),
                false => tx.send((id, Event::ButtonReleased(Button::DPadRight))),
            };
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_START) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_START != 0 {
                true => tx.send((id, Event::ButtonPressed(Button::Start))),
                false => tx.send((id, Event::ButtonReleased(Button::Start))),
            };
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_BACK) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_BACK != 0 {
                true => tx.send((id, Event::ButtonPressed(Button::Select))),
                false => tx.send((id, Event::ButtonReleased(Button::Select))),
            };
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_LEFT_THUMB) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_LEFT_THUMB != 0 {
                true => tx.send((id, Event::ButtonPressed(Button::LeftThumb))),
                false => tx.send((id, Event::ButtonReleased(Button::LeftThumb))),
            };
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_RIGHT_THUMB) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_RIGHT_THUMB != 0 {
                true => tx.send((id, Event::ButtonPressed(Button::RightThumb))),
                false => tx.send((id, Event::ButtonReleased(Button::RightThumb))),
            };
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_LEFT_SHOULDER) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_LEFT_SHOULDER != 0 {
                true => tx.send((id, Event::ButtonPressed(Button::LeftTrigger))),
                false => tx.send((id, Event::ButtonReleased(Button::LeftTrigger))),
            };
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_RIGHT_SHOULDER) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_RIGHT_SHOULDER != 0 {
                true => tx.send((id, Event::ButtonPressed(Button::RightTrigger))),
                false => tx.send((id, Event::ButtonReleased(Button::RightTrigger))),
            };
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_A) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_A != 0 {
                true => tx.send((id, Event::ButtonPressed(Button::South))),
                false => tx.send((id, Event::ButtonReleased(Button::South))),
            };
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_B) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_B != 0 {
                true => tx.send((id, Event::ButtonPressed(Button::East))),
                false => tx.send((id, Event::ButtonReleased(Button::East))),
            };
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_X) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_X != 0 {
                true => tx.send((id, Event::ButtonPressed(Button::West))),
                false => tx.send((id, Event::ButtonReleased(Button::West))),
            };
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_Y) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_Y != 0 {
                true => tx.send((id, Event::ButtonPressed(Button::North))),
                false => tx.send((id, Event::ButtonReleased(Button::North))),
            };
        }
    }
}

#[derive(Debug)]
pub struct Gamepad {
    name: String,
    uuid: Uuid,
    id: u32,
}

impl Gamepad {
    fn none() -> Self {
        Gamepad {
            name: String::new(),
            uuid: Uuid::nil(),
            id: U32_MAX,
        }
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    pub fn max_ff_effects(&self) -> usize {
        0
    }

    pub fn is_ff_supported(&self) -> bool {
        false
    }

    pub fn set_ff_gain(&mut self, gain: u16) {}
}

pub struct EventIterator<'a>(&'a mut Gilrs);

impl<'a> Iterator for EventIterator<'a> {
    type Item = (usize, Event);

    fn next(&mut self) -> Option<(usize, Event)> {
        self.0.rx.try_recv().ok().map(|(id, event)| {
            let gamepads = &mut self.0.gamepads;
            match event {
                Event::ButtonPressed(btn) => gamepads[id].state_mut().set_btn(btn, true),
                Event::ButtonReleased(btn) => gamepads[id].state_mut().set_btn(btn, false),
                Event::AxisChanged(axis, val) => gamepads[id].state_mut().set_axis(axis, val),
                Event::Connected => *gamepads[id].status_mut() = Status::Connected,
                Event::Disconnected => *gamepads[id].status_mut() = Status::Disconnected,
            };
            (id, event)
        })
    }
}

#[inline(always)]
fn is_mask_eq(l: u16, r: u16, mask: u16) -> bool {
    (l & mask != 0) == (r & mask != 0)
}

fn gamepad_new(id: u32) -> gamepad::Gamepad {
    let gamepad = Gamepad {
        name: format!("XInput Controller {}", id + 1),
        uuid: Uuid::nil(),
        id: id,
    };

    let status = unsafe {
        let mut state = mem::zeroed::<XState>();
        if xinput::XInputGetState(id, &mut state as *mut _) == ERROR_SUCCESS {
            Status::Connected
        } else {
            Status::NotObserved
        }
    };

    gamepad::Gamepad::from_inner_status(gamepad, status)
}

pub mod native_ev_codes {
    #![allow(dead_code)]
    pub const BTN_SOUTH: u16 = 0;
    pub const BTN_EAST: u16 = 1;
    pub const BTN_C: u16 = 2;
    pub const BTN_NORTH: u16 = 3;
    pub const BTN_WEST: u16 = 4;
    pub const BTN_Z: u16 = 5;
    pub const BTN_LT: u16 = 6;
    pub const BTN_RT: u16 = 7;
    pub const BTN_LT2: u16 = 8;
    pub const BTN_RT2: u16 = 9;
    pub const BTN_SELECT: u16 = 10;
    pub const BTN_START: u16 = 11;
    pub const BTN_MODE: u16 = 12;
    pub const BTN_LTHUMB: u16 = 13;
    pub const BTN_RTHUMB: u16 = 14;

    pub const BTN_DPAD_UP: u16 = 15;
    pub const BTN_DPAD_DOWN: u16 = 16;
    pub const BTN_DPAD_LEFT: u16 = 17;
    pub const BTN_DPAD_RIGHT: u16 = 18;

    pub const AXIS_LSTICKX: u16 = 0;
    pub const AXIS_LSTICKY: u16 = 1;
    pub const AXIS_LEFTZ: u16 = 2;
    pub const AXIS_RSTICKX: u16 = 3;
    pub const AXIS_RSTICKY: u16 = 4;
    pub const AXIS_RIGHTZ: u16 = 5;
    pub const AXIS_DPADX: u16 = 6;
    pub const AXIS_DPADY: u16 = 7;
    pub const AXIS_RT: u16 = 8;
    pub const AXIS_LT: u16 = 9;
    pub const AXIS_RT2: u16 = 10;
    pub const AXIS_LT2: u16 = 11;
}
