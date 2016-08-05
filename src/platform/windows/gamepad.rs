#![allow(unused_variables)]

use gamepad::{Event, Status, Axis, Button};
use uuid::Uuid;
use std::thread;
use std::mem;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;
use std::u32::MAX as U32_MAX;
use std::i16::MAX as I16_MAX;
use std::u8::MAX as U8_MAX;
use winapi::winerror::ERROR_SUCCESS;
use winapi::xinput::{XINPUT_STATE as XState, XINPUT_GAMEPAD_DPAD_UP, XINPUT_GAMEPAD_DPAD_DOWN,
XINPUT_GAMEPAD_DPAD_LEFT, XINPUT_GAMEPAD_DPAD_RIGHT, XINPUT_GAMEPAD_START, XINPUT_GAMEPAD_BACK,
XINPUT_GAMEPAD_LEFT_THUMB, XINPUT_GAMEPAD_RIGHT_THUMB, XINPUT_GAMEPAD_LEFT_SHOULDER,
XINPUT_GAMEPAD_RIGHT_SHOULDER, XINPUT_GAMEPAD_A, XINPUT_GAMEPAD_B, XINPUT_GAMEPAD_X, XINPUT_GAMEPAD_Y};

use xinput;

const EVENT_THREAD_SLEEP_TIME: u64 = 10;

#[derive(Debug)]
pub struct Gilrs {
    pub gamepads: Vec<Gamepad>,
}

impl Gilrs {
    pub fn new() -> Self {
        let mut gamepads = Vec::new();
        unsafe { xinput::XInputEnable(1) };
        for i in 0..4 {
            if let Some(gamepad) = Gamepad::try_create(i) {
                gamepads.push(gamepad);
                println!("Foung gamepda with id {}", i);
            }
        }
        Gilrs { gamepads: gamepads }
    }

    pub fn handle_hotplug(&mut self) -> Option<(Gamepad, Status)> {
        None
    }
}

#[derive(Debug)]
pub struct Gamepad {
    pub name: String,
    pub uuid: Uuid,
    id: u32,
    rx: Receiver<Event>,
}

impl Gamepad {
    /// Returns gamepad that had never existed. All actions performed on returned object are no-op.
    pub fn none() -> Self {
        Gamepad {
            name: String::new(),
            uuid: Uuid::nil(),
            id: U32_MAX,
            rx: mpsc::channel().1,
        }
    }

    fn try_create(id: u32) -> Option<Self> {
        unsafe {
            let mut xstate = mem::zeroed::<XState>();
            if xinput::XInputGetState(id, &mut xstate as *mut _) == ERROR_SUCCESS {
                let (tx, rx) = mpsc::channel();

                thread::spawn(move || {
                    let mut prev_state = xstate;
                    let mut state = mem::zeroed::<XState>();
                    loop {
                        if xinput::XInputGetState(id, &mut state as &mut _) == ERROR_SUCCESS {
                            if state.dwPacketNumber != prev_state.dwPacketNumber {
                                {
                                    let (g, pg) = (&state.Gamepad, &prev_state.Gamepad);
                                    if g.bLeftTrigger != pg.bLeftTrigger {
                                        let _ = tx.send(Event::AxisChanged(Axis::LeftTrigger2,
                                                                   g.bLeftTrigger as f32 /
                                                                   U8_MAX as f32));
                                    }
                                    if g.bRightTrigger != pg.bRightTrigger {
                                        let _ = tx.send(Event::AxisChanged(Axis::RightTrigger2,
                                                                   g.bRightTrigger as f32 /
                                                                   U8_MAX as f32));
                                    }
                                    if g.sThumbLX != pg.sThumbLX {
                                        let _ = tx.send(Event::AxisChanged(Axis::LeftStickX,
                                                                   g.sThumbLX as f32 /
                                                                   I16_MAX as f32));
                                    }
                                    if g.sThumbLY != pg.sThumbLY {
                                        let _ = tx.send(Event::AxisChanged(Axis::LeftStickY,
                                                                   g.sThumbLY as f32 /
                                                                   I16_MAX as f32));
                                    }
                                    if g.sThumbRX != pg.sThumbRX {
                                        let _ = tx.send(Event::AxisChanged(Axis::RightStickX,
                                                                   g.sThumbRX as f32 /
                                                                   I16_MAX as f32));
                                    }
                                    if g.sThumbRY != pg.sThumbRY {
                                        let _ = tx.send(Event::AxisChanged(Axis::RightStickY,
                                                                   g.sThumbRY as f32 /
                                                                   I16_MAX as f32));
                                    }
                                    if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_DPAD_UP) {
                                        let _ = match g.wButtons & XINPUT_GAMEPAD_DPAD_UP != 0 {
                                            true => tx.send(Event::ButtonPressed(Button::DPadUp)),
                                            false => tx.send(Event::ButtonReleased(Button::DPadUp)),
                                        };
                                    }
                                    if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_DPAD_DOWN) {
                                        let _ = match g.wButtons & XINPUT_GAMEPAD_DPAD_DOWN != 0 {
                                            true => tx.send(Event::ButtonPressed(Button::DPadDown)),
                                            false => tx.send(Event::ButtonReleased(Button::DPadDown)),
                                        };
                                    }
                                    if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_DPAD_LEFT) {
                                        let _ = match g.wButtons & XINPUT_GAMEPAD_DPAD_LEFT != 0 {
                                            true => tx.send(Event::ButtonPressed(Button::DPadLeft)),
                                            false => tx.send(Event::ButtonReleased(Button::DPadLeft)),
                                        };
                                    }
                                    if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_DPAD_RIGHT) {
                                        let _ = match g.wButtons & XINPUT_GAMEPAD_DPAD_RIGHT != 0 {
                                            true => tx.send(Event::ButtonPressed(Button::DPadRight)),
                                            false => tx.send(Event::ButtonReleased(Button::DPadRight)),
                                        };
                                    }
                                    if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_START) {
                                        let _ = match g.wButtons & XINPUT_GAMEPAD_START != 0 {
                                            true => tx.send(Event::ButtonPressed(Button::Start)),
                                            false => tx.send(Event::ButtonReleased(Button::Start)),
                                        };
                                    }
                                    if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_BACK) {
                                        let _ = match g.wButtons & XINPUT_GAMEPAD_BACK != 0 {
                                            true => tx.send(Event::ButtonPressed(Button::Select)),
                                            false => tx.send(Event::ButtonReleased(Button::Select)),
                                        };
                                    }
                                    if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_LEFT_THUMB) {
                                        let _ = match g.wButtons & XINPUT_GAMEPAD_LEFT_THUMB != 0 {
                                            true => tx.send(Event::ButtonPressed(Button::LeftThumb)),
                                            false => tx.send(Event::ButtonReleased(Button::LeftThumb)),
                                        };
                                    }
                                    if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_RIGHT_THUMB) {
                                        let _ = match g.wButtons & XINPUT_GAMEPAD_RIGHT_THUMB != 0 {
                                            true => tx.send(Event::ButtonPressed(Button::RightThumb)),
                                            false => tx.send(Event::ButtonReleased(Button::RightThumb)),
                                        };
                                    }
                                    if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_LEFT_SHOULDER) {
                                        let _ = match g.wButtons & XINPUT_GAMEPAD_LEFT_SHOULDER != 0 {
                                            true => tx.send(Event::ButtonPressed(Button::LeftTrigger)),
                                            false => tx.send(Event::ButtonReleased(Button::LeftTrigger)),
                                        };
                                    }
                                    if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_RIGHT_SHOULDER) {
                                        let _ = match g.wButtons & XINPUT_GAMEPAD_RIGHT_SHOULDER != 0 {
                                            true => tx.send(Event::ButtonPressed(Button::RightTrigger)),
                                            false => tx.send(Event::ButtonReleased(Button::RightTrigger)),
                                        };
                                    }
                                    if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_A) {
                                        let _ = match g.wButtons & XINPUT_GAMEPAD_A != 0 {
                                            true => tx.send(Event::ButtonPressed(Button::South)),
                                            false => tx.send(Event::ButtonReleased(Button::South)),
                                        };
                                    }
                                    if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_B) {
                                        let _ = match g.wButtons & XINPUT_GAMEPAD_B != 0 {
                                            true => tx.send(Event::ButtonPressed(Button::East)),
                                            false => tx.send(Event::ButtonReleased(Button::East)),
                                        };
                                    }
                                    if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_X) {
                                        let _ = match g.wButtons & XINPUT_GAMEPAD_X != 0 {
                                            true => tx.send(Event::ButtonPressed(Button::West)),
                                            false => tx.send(Event::ButtonReleased(Button::West)),
                                        };
                                    }
                                    if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_Y) {
                                        let _ = match g.wButtons & XINPUT_GAMEPAD_Y != 0 {
                                            true => tx.send(Event::ButtonPressed(Button::North)),
                                            false => tx.send(Event::ButtonReleased(Button::North)),
                                        };
                                    }
                                }
                                prev_state = state;
                            }
                        }
                        thread::sleep(Duration::from_millis(EVENT_THREAD_SLEEP_TIME));
                    }
                });

                Some(Gamepad {
                    name: String::new(),
                    uuid: Uuid::nil(),
                    id: id,
                    rx: rx,
                })
            } else {
                None
            }
        }
    }

    pub fn eq_disconnect(&self, other: &Self) -> bool {
        false
    }

    pub fn event(&mut self) -> Option<Event> {
        self.rx.try_recv().ok()
    }

    pub fn disconnect(&mut self) {}

    pub fn max_ff_effects(&self) -> usize {
        0
    }

    pub fn is_ff_supported(&self) -> bool {
        false
    }

    pub fn set_ff_gain(&mut self, gain: u16) {}
}

#[inline(always)]
fn is_mask_eq(l: u16, r: u16, mask: u16) -> bool {
    (l & mask != 0) == (r & mask != 0)
}

impl PartialEq for Gamepad {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
    }
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
