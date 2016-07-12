use ff::{EffectData, Waveform};
use super::gamepad::Gamepad;
use ioctl::{ff_effect, input_event};
use ioctl;
use libc as c;
use std::mem;

#[derive(Debug)]
pub struct Effect<'a> {
    id: i16,
    gamepad: &'a Gamepad,
}

impl<'a> Effect<'a> {
    pub fn new(gamepad: &'a Gamepad, data: EffectData) -> Option<Self> {
        let mut data: ff_effect = data.into();
        let res = unsafe { ioctl::eviocsff(gamepad.fd(), &mut data as *mut _) };
        if res == -1 {
            None
        } else {
            Some(Effect {
                id: data.id,
                gamepad: gamepad,
            })
        }
    }

    pub fn upload(&mut self, data: EffectData) -> Option<()> {
        let mut data: ff_effect = data.into();
        data.id = self.id;
        let res = unsafe { ioctl::eviocsff(self.gamepad.fd(), &mut data as *mut _) };
        if res == -1 {
            None
        } else {
            Some(())
        }
    }

    pub fn play(&mut self, n: u16) {
        let ev = input_event {
            _type: EV_FF,
            code: self.id as u16,
            value: n as i32,
            time: unsafe { mem::uninitialized() },
        };
        unsafe { c::write(self.gamepad.fd(), mem::transmute(&ev), 24) };
    }

    pub fn stop(&mut self) {
        self.play(0)
    }
}

impl<'a> Drop for Effect<'a> {
    fn drop(&mut self) {
        unsafe {
            // bug in ioctl crate, second argument is i32 not pointer to i32
            ioctl::eviocrmff(self.gamepad.fd(), mem::transmute(self.id as isize));
        }
    }
}

impl Into<ff_effect> for EffectData {
    fn into(self) -> ff_effect {
        let mut effect = ff_effect {
            _type: FF_PERIODIC,
            id: -1,
            direction: self.direction.angle,
            trigger: unsafe { mem::transmute(self.trigger) },
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

const EV_FF: u16 = 0x15;

const FF_PERIODIC: u16 = 0x51;
const FF_SQUARE: u16 = 0x58;
const FF_TRIANGLE: u16 = 0x59;
const FF_SINE: u16 = 0x5a;
