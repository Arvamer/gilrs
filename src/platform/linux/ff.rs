// Copyright 2016 GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
use ff::{EffectData, Waveform, Trigger, Error};
use super::gamepad::Gamepad;
use ioctl::{ff_effect, input_event};
use ioctl;
use libc as c;
use std::mem;
use constants;

#[derive(Debug)]
pub struct Effect {
    id: i16,
    fd: i32,
}

impl Effect {
    pub fn new(gamepad: &Gamepad, data: EffectData) -> Result<Self, Error> {
        let mut data: ff_effect = data.into();
        let res = unsafe { ioctl::eviocsff(gamepad.fd(), &mut data as *mut _) };
        if res == -1 {
            Err(Error::EffectNotSupported)
        } else {
            Ok(Effect {
                id: data.id,
                fd: gamepad.fd(),
            })
        }
    }

    pub fn upload(&mut self, data: EffectData) -> Result<(), Error> {
        let mut data: ff_effect = data.into();
        data.id = self.id;
        let res = unsafe { ioctl::eviocsff(self.fd, &mut data as *mut _) };
        if res == -1 { Err(Error::EffectNotSupported) } else { Ok(()) }
    }

    pub fn play(&mut self, n: u16) {
        let ev = input_event {
            _type: EV_FF,
            code: self.id as u16,
            value: n as i32,
            time: unsafe { mem::uninitialized() },
        };
        unsafe { c::write(self.fd, mem::transmute(&ev), 24) };
    }

    pub fn stop(&mut self) {
        self.play(0)
    }
}

impl Drop for Effect {
    fn drop(&mut self) {
        unsafe {
            // bug in ioctl crate, second argument is i32 not pointer to i32
            ioctl::eviocrmff(self.fd, mem::transmute(self.id as isize));
        }
    }
}

impl Into<ff_effect> for EffectData {
    fn into(self) -> ff_effect {
        let mut effect = ff_effect {
            _type: FF_PERIODIC,
            id: -1,
            direction: self.direction.angle,
            trigger: self.trigger.into(),
            replay: unsafe { mem::transmute(self.replay) },
            u: unsafe { mem::uninitialized() },
        };
        unsafe {
            let mut periodic = effect.u.periodic();
            (*periodic).waveform = self.wave.into();
            (*periodic).period = self.period;
            (*periodic).magnitude = self.magnitude;
            (*periodic).offset = self.offset;
            (*periodic).phase = self.phase;
            (*periodic).envelope = mem::transmute(self.envelope);
        }
        effect
    }
}

impl Into<u16> for Waveform {
    fn into(self) -> u16 {
        match self {
            Waveform::Square => FF_SQUARE,
            Waveform::Triangle => FF_TRIANGLE,
            Waveform::Sine => FF_SINE,
        }
    }
}

impl Into<ioctl::ff_trigger> for Trigger {
    fn into(self) -> ioctl::ff_trigger {
        let mut val = self.button as u16;
        if val >= constants::BTN_SOUTH && val <= constants::BTN_RTHUMB {
            val += BTN_GAMEPAD;
        } else if val >= constants::BTN_DPAD_UP && val <= constants::BTN_DPAD_RIGHT {
            val += BTN_DPAD_UP - constants::BTN_DPAD_UP;
        } else {
            val = 0;
        };
        ioctl::ff_trigger {
            button: val,
            interval: self.interval,
        }
    }
}

const EV_FF: u16 = 0x15;

const FF_PERIODIC: u16 = 0x51;
const FF_SQUARE: u16 = 0x58;
const FF_TRIANGLE: u16 = 0x59;
const FF_SINE: u16 = 0x5a;
const BTN_GAMEPAD: u16 = 0x130;
const BTN_DPAD_UP: u16 = 0x220;
