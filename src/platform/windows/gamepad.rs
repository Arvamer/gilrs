// Copyright 2016 GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use gamepad::{self, Event, Status, Axis, Button, PowerInfo, GamepadImplExt, Deadzones, MappingSource};
use mapping::{MappingData, MappingError};
use ff::Error;
use super::ff::{FfMessage, FfMessageType, Device, self};
use uuid::Uuid;
use std::time::Duration;
use std::{thread, mem, u32, i16, u8, u16};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender};
use winapi::winerror::{ERROR_SUCCESS, ERROR_DEVICE_NOT_CONNECTED};
use winapi::xinput::{XINPUT_STATE as XState, XINPUT_GAMEPAD_DPAD_UP, XINPUT_GAMEPAD_DPAD_DOWN,
                     XINPUT_GAMEPAD_DPAD_LEFT, XINPUT_GAMEPAD_DPAD_RIGHT, XINPUT_GAMEPAD_START,
                     XINPUT_GAMEPAD_BACK, XINPUT_GAMEPAD_LEFT_THUMB, XINPUT_GAMEPAD_RIGHT_THUMB,
                     XINPUT_GAMEPAD_LEFT_SHOULDER, XINPUT_GAMEPAD_RIGHT_SHOULDER, XINPUT_GAMEPAD_A,
                     XINPUT_GAMEPAD_B, XINPUT_GAMEPAD_X, XINPUT_GAMEPAD_Y,
                     XINPUT_GAMEPAD as XGamepad, XINPUT_BATTERY_INFORMATION as XBatteryInfo,
                     self as xi};

use xinput;

// Chosen by dice roll ;)
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
        let (fftx, ffrx) = mpsc::sync_channel(4);
        let gamepads = [gamepad_new(0, fftx.clone()),
                        gamepad_new(1, fftx.clone()),
                        gamepad_new(2, fftx.clone()),
                        gamepad_new(3, fftx)];
        let connected = [gamepads[0].is_connected(),
                         gamepads[1].is_connected(),
                         gamepads[2].is_connected(),
                         gamepads[3].is_connected()];
        unsafe { xinput::XInputEnable(1) };
        let (tx, rx) = mpsc::channel();
        Self::spawn_thread(tx, ffrx, connected);
        Gilrs {
            gamepads: gamepads,
            rx: rx,
            not_observed: gamepad::Gamepad::from_inner_status(Gamepad::none(),
                                                              Status::NotObserved,
                                                              deadzones()),
        }
    }

    pub fn with_mappings(_sdl_mapping: &str) -> Self {
        Self::new()
    }

    pub fn next_event(&mut self) -> Option<(usize, Event)> {
        self.rx.try_recv().ok()
    }

    pub fn gamepad(&self, id: usize) -> &gamepad::Gamepad {
        self.gamepads.get(id).unwrap_or(&self.not_observed)
    }

    pub fn gamepad_mut(&mut self, id: usize) -> &mut gamepad::Gamepad {
        self.gamepads.get_mut(id).unwrap_or(&mut self.not_observed)
    }

    pub fn last_gamepad_hint(&self) -> usize {
        self.gamepads.len()
    }

    fn spawn_thread(tx: Sender<(usize, Event)>, ffrx: Receiver<FfMessage>, connected: [bool; 4]) {
        thread::spawn(move || unsafe {
            let mut prev_state = mem::zeroed::<XState>();
            let mut state = mem::zeroed::<XState>();
            let mut connected = connected;
            let mut counter = 0;

            let mut ff = [None; 4];
            for (cn, i) in connected.iter().zip(0..) {
                if *cn {
                    ff[i as usize] = Some(Device::new(i));
                }
            }

            loop {
                for id in 0..4 {
                    if *connected.get_unchecked(id) ||
                       counter % ITERATIONS_TO_CHECK_IF_CONNECTED == 0 {
                        let val = xinput::XInputGetState(id as u32, &mut state);

                        if val == ERROR_SUCCESS {
                            if !connected.get_unchecked(id) {
                                *connected.get_unchecked_mut(id) = true;
                                *ff.get_unchecked_mut(id) = Some(Device::new(id as u8));
                                let _ = tx.send((id, Event::Connected));
                            }

                            if state.dwPacketNumber != prev_state.dwPacketNumber {
                                Self::compare_state(id, &state.Gamepad, &prev_state.Gamepad, &tx);
                                prev_state = state;
                            }
                        } else if val == ERROR_DEVICE_NOT_CONNECTED &&
                                  *connected.get_unchecked(id) {
                            *connected.get_unchecked_mut(id) = false;
                            *ff.get_unchecked_mut(id) = None;
                            let _ = tx.send((id, Event::Disconnected));
                        }
                    }
                }

                Self::recv_ff_events(&ffrx, &mut ff);

                for dev in ff.iter_mut() {
                    if let Some(dev) = dev.as_mut() {
                        dev.combine_and_play();
                    }
                }

                counter = counter.wrapping_add(1);
                thread::sleep(Duration::from_millis(EVENT_THREAD_SLEEP_TIME));
            }
        });
    }

    fn recv_ff_events(rx: &Receiver<FfMessage>, ff: &mut [Option<Device>]) {
        while let Ok(msg) = rx.try_recv() {
            if let Some(dev) = ff[msg.id as usize].as_mut() {
                match msg.kind {
                    FfMessageType::Create(data) => dev.create(msg.idx, data),
                    FfMessageType::Play(n) => dev.play(msg.idx, n),
                    FfMessageType::Stop => dev.stop(msg.idx),
                    FfMessageType::Drop => dev.drop(msg.idx),
                    FfMessageType::ChangeGain(new) => dev.set_gain(new),
                }
            } else {
                error!("Received force feedback message for disconnected gamepad.")
            }
        }
    }

    fn compare_state(id: usize, g: &XGamepad, pg: &XGamepad, tx: &Sender<(usize, Event)>) {
        if g.bLeftTrigger != pg.bLeftTrigger {
            let _ = tx.send((id,
                             Event::AxisChanged(Axis::LeftTrigger2,
                                                g.bLeftTrigger as f32 / u8::MAX as f32,
                                                4)));
        }
        if g.bRightTrigger != pg.bRightTrigger {
            let _ = tx.send((id,
                             Event::AxisChanged(Axis::RightTrigger2,
                                                g.bRightTrigger as f32 / u8::MAX as f32,
                                                5)));
        }
        if g.sThumbLX != pg.sThumbLX {
            let _ = tx.send((id,
                             Event::AxisChanged(Axis::LeftStickX,
                                                g.sThumbLX as f32 / i16::MAX as f32,
                                                0)));
        }
        if g.sThumbLY != pg.sThumbLY {
            let _ = tx.send((id,
                             Event::AxisChanged(Axis::LeftStickY,
                                                g.sThumbLY as f32 / i16::MAX as f32,
                                                1)));
        }
        if g.sThumbRX != pg.sThumbRX {
            let _ = tx.send((id,
                             Event::AxisChanged(Axis::RightStickX,
                                                g.sThumbRX as f32 / i16::MAX as f32,
                                                2)));
        }
        if g.sThumbRY != pg.sThumbRY {
            let _ = tx.send((id,
                             Event::AxisChanged(Axis::RightStickY,
                                                g.sThumbRY as f32 / i16::MAX as f32,
                                                3)));
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_DPAD_UP) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_DPAD_UP != 0 {
                true => tx.send((id, Event::ButtonPressed(Button::DPadUp, XINPUT_GAMEPAD_DPAD_UP))),
                false => {
                    tx.send((id, Event::ButtonReleased(Button::DPadUp, XINPUT_GAMEPAD_DPAD_UP)))
                }
            };
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_DPAD_DOWN) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_DPAD_DOWN != 0 {
                true => {
                    tx.send((id, Event::ButtonPressed(Button::DPadDown, XINPUT_GAMEPAD_DPAD_DOWN)))
                }
                false => {
                    tx.send((id, Event::ButtonReleased(Button::DPadDown, XINPUT_GAMEPAD_DPAD_DOWN)))
                }
            };
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_DPAD_LEFT) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_DPAD_LEFT != 0 {
                true => {
                    tx.send((id, Event::ButtonPressed(Button::DPadLeft, XINPUT_GAMEPAD_DPAD_LEFT)))
                }
                false => {
                    tx.send((id, Event::ButtonReleased(Button::DPadLeft, XINPUT_GAMEPAD_DPAD_LEFT)))
                }
            };
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_DPAD_RIGHT) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_DPAD_RIGHT != 0 {
                true => {
                    tx.send((id,
                             Event::ButtonPressed(Button::DPadRight, XINPUT_GAMEPAD_DPAD_RIGHT)))
                }
                false => {
                    tx.send((id,
                             Event::ButtonReleased(Button::DPadRight, XINPUT_GAMEPAD_DPAD_RIGHT)))
                }
            };
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_START) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_START != 0 {
                true => tx.send((id, Event::ButtonPressed(Button::Start, XINPUT_GAMEPAD_START))),
                false => tx.send((id, Event::ButtonReleased(Button::Start, XINPUT_GAMEPAD_START))),
            };
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_BACK) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_BACK != 0 {
                true => tx.send((id, Event::ButtonPressed(Button::Select, XINPUT_GAMEPAD_BACK))),
                false => tx.send((id, Event::ButtonReleased(Button::Select, XINPUT_GAMEPAD_BACK))),
            };
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_LEFT_THUMB) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_LEFT_THUMB != 0 {
                true => {
                    tx.send((id,
                             Event::ButtonPressed(Button::LeftThumb, XINPUT_GAMEPAD_LEFT_THUMB)))
                }
                false => {
                    tx.send((id,
                             Event::ButtonReleased(Button::LeftThumb, XINPUT_GAMEPAD_LEFT_THUMB)))
                }
            };
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_RIGHT_THUMB) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_RIGHT_THUMB != 0 {
                true => {
                    tx.send((id,
                             Event::ButtonPressed(Button::RightThumb, XINPUT_GAMEPAD_RIGHT_THUMB)))
                }
                false => {
                    tx.send((id,
                             Event::ButtonReleased(Button::RightThumb, XINPUT_GAMEPAD_RIGHT_THUMB)))
                }
            };
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_LEFT_SHOULDER) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_LEFT_SHOULDER != 0 {
                true => {
                    tx.send((id,
                             Event::ButtonPressed(Button::LeftTrigger,
                                                  XINPUT_GAMEPAD_LEFT_SHOULDER)))
                }
                false => {
                    tx.send((id,
                             Event::ButtonReleased(Button::LeftTrigger,
                                                   XINPUT_GAMEPAD_LEFT_SHOULDER)))
                }
            };
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_RIGHT_SHOULDER) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_RIGHT_SHOULDER != 0 {
                true => {
                    tx.send((id,
                             Event::ButtonPressed(Button::RightTrigger,
                                                  XINPUT_GAMEPAD_RIGHT_SHOULDER)))
                }
                false => {
                    tx.send((id,
                             Event::ButtonReleased(Button::RightTrigger,
                                                   XINPUT_GAMEPAD_RIGHT_SHOULDER)))
                }
            };
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_A) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_A != 0 {
                true => tx.send((id, Event::ButtonPressed(Button::South, XINPUT_GAMEPAD_A))),
                false => tx.send((id, Event::ButtonReleased(Button::South, XINPUT_GAMEPAD_A))),
            };
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_B) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_B != 0 {
                true => tx.send((id, Event::ButtonPressed(Button::East, XINPUT_GAMEPAD_B))),
                false => tx.send((id, Event::ButtonReleased(Button::East, XINPUT_GAMEPAD_B))),
            };
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_X) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_X != 0 {
                true => tx.send((id, Event::ButtonPressed(Button::West, XINPUT_GAMEPAD_X))),
                false => tx.send((id, Event::ButtonReleased(Button::West, XINPUT_GAMEPAD_X))),
            };
        }
        if !is_mask_eq(g.wButtons, pg.wButtons, XINPUT_GAMEPAD_Y) {
            let _ = match g.wButtons & XINPUT_GAMEPAD_Y != 0 {
                true => tx.send((id, Event::ButtonPressed(Button::North, XINPUT_GAMEPAD_Y))),
                false => tx.send((id, Event::ButtonReleased(Button::North, XINPUT_GAMEPAD_Y))),
            };
        }
    }
}

#[derive(Debug)]
pub struct Gamepad {
    name: String,
    uuid: Uuid,
    id: u32,
    ff_sender: Option<SyncSender<FfMessage>>,
    ff_effect_idxs: u16,
}

impl Gamepad {
    fn none() -> Self {
        Gamepad {
            name: String::new(),
            uuid: Uuid::nil(),
            id: u32::MAX,
            ff_sender: None,
            ff_effect_idxs: 0,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    pub fn power_info(&self) -> PowerInfo {
        unsafe {
            let mut binfo = mem::uninitialized::<XBatteryInfo>();
            if xinput::XInputGetBatteryInformation(self.id,
                                                   xi::BATTERY_DEVTYPE_GAMEPAD,
                                                   &mut binfo) == ERROR_SUCCESS {
                match binfo.BatteryType {
                    xi::BATTERY_TYPE_WIRED => PowerInfo::Wired,
                    xi::BATTERY_TYPE_ALKALINE |
                    xi::BATTERY_TYPE_NIMH => {
                        let lvl = match binfo.BatteryLevel {
                            xi::BATTERY_LEVEL_EMPTY => 0,
                            xi::BATTERY_LEVEL_LOW => 33,
                            xi::BATTERY_LEVEL_MEDIUM => 67,
                            xi::BATTERY_LEVEL_FULL => 100,
                            _ => unreachable!(),
                        };
                        if lvl == 100 { PowerInfo::Charged } else { PowerInfo::Discharging(lvl) }
                    }
                    _ => PowerInfo::Unknown,
                }
            } else {
                PowerInfo::Unknown
            }
        }
    }

    pub fn mapping_source(&self) -> MappingSource {
        MappingSource::Driver
    }

    pub fn set_mapping(&mut self,
                       _mapping: &MappingData,
                       _strict: bool,
                       _name: Option<&str>)
                       -> Result<String, MappingError> {
        Err(MappingError::NotImplemented)
    }

    pub fn max_ff_effects(&self) -> usize {
        ff::MAX_EFFECTS
    }

    pub fn is_ff_supported(&self) -> bool {
        true
    }

    pub fn set_ff_gain(&mut self, gain: u16) -> Result<(), Error> {
        let gain = gain as f32 / u16::MAX as f32;
        let msg = FfMessage {
            id: self.id as u8,
            idx: 0,
            kind: FfMessageType::ChangeGain(gain),
        };

        self.ff_sender.as_ref().expect("Attempt to get ff_sender from fake gamepad.").try_send(msg)?;
        Ok(())
    }

    pub fn ff_sender(&self) -> &SyncSender<FfMessage> {
        // This function should be only called on "real" gamepads with ff_sender. If this panic,
        // pleas open an issue—it's bug in library.
        self.ff_sender.as_ref().expect("Attempt to get ff_sender from fake gamepad.")
    }

    pub fn ff_effect_idxs_ptr(&self) -> *mut u16 {
        &self.ff_effect_idxs as *const _ as *mut _
    }

    pub unsafe fn next_ff_idx(ff_effect_idxs: *mut u16) -> Option<u8> {
        for i in 0..ff::MAX_EFFECTS {
            if (*ff_effect_idxs >> i) & 1 == 0 {
                *ff_effect_idxs |= 1 << i;
                return Some(i as u8);
            }
        }
        None
    }

    pub unsafe fn free_ff_idx(ff_effect_idxs: *mut u16, idx: u8) {
        *ff_effect_idxs &= !(1 << idx);
    }

    pub fn id(&self) -> u8 {
        self.id as u8
    }
}

#[inline(always)]
fn is_mask_eq(l: u16, r: u16, mask: u16) -> bool {
    (l & mask != 0) == (r & mask != 0)
}

fn gamepad_new(id: u32, ff_sender: SyncSender<FfMessage>) -> gamepad::Gamepad {
    let gamepad = Gamepad {
        name: format!("XInput Controller {}", id + 1),
        uuid: Uuid::nil(),
        id: id,
        ff_sender: Some(ff_sender),
        ff_effect_idxs: 0,
    };

    let status = unsafe {
        let mut state = mem::zeroed::<XState>();
        if xinput::XInputGetState(id, &mut state) == ERROR_SUCCESS {
            Status::Connected
        } else {
            Status::NotObserved
        }
    };

    gamepad::Gamepad::from_inner_status(gamepad, status, deadzones())
}

fn deadzones() -> Deadzones {
    Deadzones {
        right_stick: xi::XINPUT_GAMEPAD_RIGHT_THUMB_DEADZONE as f32 / 65534.0,
        left_stick: xi::XINPUT_GAMEPAD_LEFT_THUMB_DEADZONE as f32 / 65534.0,
        left_trigger2: xi::XINPUT_GAMEPAD_TRIGGER_THRESHOLD as f32 / 255.0,
        ..Default::default()
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
