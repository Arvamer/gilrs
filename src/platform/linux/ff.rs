// Copyright 2016 GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use std::fs::File;
use std::io::{Write, Result as IoResult, Error as IoError, ErrorKind};
use std::os::unix::io::AsRawFd;
use std::{mem, slice};

use ff::TICK_DURATION;
use super::ioctl::{self, ff_effect, input_event, ff_replay, ff_rumble_effect};

#[derive(Debug)]
pub struct Device {
    effect: i16,
    file: File,
}

impl Device {
    pub fn new(path: &str) -> IoResult<Self>  {
        let file = File::create(path)?;
        let mut effect = ff_effect {
            type_: FF_RUMBLE,
            id: -1,
            direction: 0,
            trigger: Default::default(),
            replay: Default::default(),
            u: Default::default(),
        };
        let res = unsafe { ioctl::eviocsff(file.as_raw_fd(), &mut effect) };
        if res.is_err() {
            Err(IoError::new(ErrorKind::Other, "Failed to create effect"))
        } else {
            Ok(Device {
                effect: effect.id,
                file: file,
            })
        }
    }

    pub(crate) fn set_ff_state(&mut self, strong: u16, weak: u16) {
       let mut effect = ff_effect {
            type_: FF_RUMBLE,
            id: self.effect,
            direction: 0,
            trigger: Default::default(),
            replay: ff_replay { delay: 0, length: TICK_DURATION as u16 * 2 },
            u: Default::default(),
        };

        let res = unsafe {
            let rumble = &mut effect.u as *mut _ as *mut ff_rumble_effect;
            (*rumble).strong_magnitude = strong;
            (*rumble).weak_magnitude = weak;
            ioctl::eviocsff(self.file.as_raw_fd(), &mut effect)
        };

        if res.is_err() {
            unimplemented!();
        } else {
            let ev = input_event {
                type_: EV_FF,
                code: self.effect as u16,
                value: 1,
                time: unsafe { mem::uninitialized() },
            };
            let size = mem::size_of::<input_event>();
            let s = unsafe { slice::from_raw_parts(&ev as *const _ as *const u8, size) };
            match self.file.write(s) {
                Ok(s) if s == size => (),
                Ok(_) => unreachable!(),
                Err(e) => error!("Failed to set ff state: {}", e),
            }
        }
    }
}

const EV_FF: u16 = 0x15;
const FF_RUMBLE: u16 = 0x50;
