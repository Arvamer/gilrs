// Copyright 2016-2018 Mateusz Sieczko and other GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
#![allow(unused_variables)]

use super::hid::*;
use super::FfDevice;
use uuid::Uuid;
use {AxisInfo, Event, PlatformError, PowerInfo};

use std::fmt::{Display, Formatter, Result as FmtResult};

#[derive(Debug)]
pub struct Gilrs {}

impl Gilrs {
    pub(crate) fn new() -> Result<Self, PlatformError> {
        Err(PlatformError::NotImplemented(Gilrs {}))
    }

    pub(crate) fn next_event(&mut self) -> Option<Event> {
        None
    }

    pub fn gamepad(&self, id: usize) -> Option<&Gamepad> {
        None
    }

    /// Returns index greater than index of last connected gamepad.
    pub fn last_gamepad_hint(&self) -> usize {
        0
    }
}

#[derive(Debug)]
pub struct Gamepad {
    _priv: (),
}

impl Gamepad {
    pub fn name(&self) -> &str {
        ""
    }

    pub fn uuid(&self) -> Uuid {
        Uuid::nil()
    }

    pub fn power_info(&self) -> PowerInfo {
        PowerInfo::Unknown
    }

    pub fn is_ff_supported(&self) -> bool {
        false
    }

    /// Creates Ffdevice corresponding to this gamepad.
    pub fn ff_device(&self) -> Option<FfDevice> {
        Some(FfDevice)
    }

    pub fn buttons(&self) -> &[EvCode] {
        &[]
    }

    pub fn axes(&self) -> &[EvCode] {
        &[]
    }

    pub(crate) fn axis_info(&self, nec: EvCode) -> Option<&AxisInfo> {
        None
    }

    pub fn is_connected(&self) -> bool {
        false
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct EvCode {
    page: u32,
    usage: u32,
}

impl EvCode {
    fn new(page: u32, usage: u32) -> Self {
        EvCode { page, usage }
    }

    pub fn into_u32(self) -> u32 {
        self.page << 16 | self.usage
    }
}

impl From<IOHIDElement> for ::EvCode {
    fn from(e: IOHIDElement) -> Self {
        ::EvCode(EvCode {
            page: e.get_page(),
            usage: e.get_usage(),
        })
    }
}

impl Display for EvCode {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self.page {
            PAGE_GENERIC_DESKTOP => f.write_str("GENERIC_DESKTOP")?,
            PAGE_BUTTON => f.write_str("BUTTON")?,
            page => f.write_fmt(format_args!("PAGE_{}", page))?,
        }
        f.write_fmt(format_args!("({})", self.usage))
    }
}

pub mod native_ev_codes {
    use super::*;

    pub const AXIS_LSTICKX: EvCode = EvCode {
        page: super::PAGE_GENERIC_DESKTOP,
        usage: super::USAGE_AXIS_LSTICKX,
    };
    pub const AXIS_LSTICKY: EvCode = EvCode {
        page: super::PAGE_GENERIC_DESKTOP,
        usage: super::USAGE_AXIS_LSTICKY,
    };
    pub const AXIS_LEFTZ: EvCode = EvCode {
        page: super::PAGE_GENERIC_DESKTOP,
        usage: super::USAGE_AXIS_LEFTZ,
    };
    pub const AXIS_RSTICKX: EvCode = EvCode {
        page: super::PAGE_GENERIC_DESKTOP,
        usage: super::USAGE_AXIS_RSTICKX,
    };
    pub const AXIS_RSTICKY: EvCode = EvCode {
        page: super::PAGE_GENERIC_DESKTOP,
        usage: super::USAGE_AXIS_RSTICKY,
    };
    pub const AXIS_RIGHTZ: EvCode = EvCode {
        page: super::PAGE_GENERIC_DESKTOP,
        usage: super::USAGE_AXIS_RIGHTZ,
    };
    pub const AXIS_DPADX: EvCode = EvCode {
        page: super::PAGE_GENERIC_DESKTOP,
        usage: super::USAGE_AXIS_DPADX,
    };
    pub const AXIS_DPADY: EvCode = EvCode {
        page: super::PAGE_GENERIC_DESKTOP,
        usage: super::USAGE_AXIS_DPADY,
    };
    pub const AXIS_RT: EvCode = EvCode {
        page: super::PAGE_GENERIC_DESKTOP,
        usage: super::USAGE_AXIS_RT,
    };
    pub const AXIS_LT: EvCode = EvCode {
        page: super::PAGE_GENERIC_DESKTOP,
        usage: super::USAGE_AXIS_LT,
    };
    pub const AXIS_RT2: EvCode = EvCode {
        page: super::PAGE_GENERIC_DESKTOP,
        usage: super::USAGE_AXIS_RT2,
    };
    pub const AXIS_LT2: EvCode = EvCode {
        page: super::PAGE_GENERIC_DESKTOP,
        usage: super::USAGE_AXIS_LT2,
    };

    pub const BTN_SOUTH: EvCode = EvCode {
        page: super::PAGE_BUTTON,
        usage: super::USAGE_BTN_SOUTH,
    };
    pub const BTN_EAST: EvCode = EvCode {
        page: super::PAGE_BUTTON,
        usage: super::USAGE_BTN_EAST,
    };
    pub const BTN_C: EvCode = EvCode {
        page: super::PAGE_BUTTON,
        usage: super::USAGE_BTN_C,
    };
    pub const BTN_NORTH: EvCode = EvCode {
        page: super::PAGE_BUTTON,
        usage: super::USAGE_BTN_NORTH,
    };
    pub const BTN_WEST: EvCode = EvCode {
        page: super::PAGE_BUTTON,
        usage: super::USAGE_BTN_WEST,
    };
    pub const BTN_Z: EvCode = EvCode {
        page: super::PAGE_BUTTON,
        usage: super::USAGE_BTN_Z,
    };
    pub const BTN_LT: EvCode = EvCode {
        page: super::PAGE_BUTTON,
        usage: super::USAGE_BTN_LT,
    };
    pub const BTN_RT: EvCode = EvCode {
        page: super::PAGE_BUTTON,
        usage: super::USAGE_BTN_RT,
    };
    pub const BTN_LT2: EvCode = EvCode {
        page: super::PAGE_BUTTON,
        usage: super::USAGE_BTN_LT2,
    };
    pub const BTN_RT2: EvCode = EvCode {
        page: super::PAGE_BUTTON,
        usage: super::USAGE_BTN_RT2,
    };
    pub const BTN_SELECT: EvCode = EvCode {
        page: super::PAGE_BUTTON,
        usage: super::USAGE_BTN_SELECT,
    };
    pub const BTN_START: EvCode = EvCode {
        page: super::PAGE_BUTTON,
        usage: super::USAGE_BTN_START,
    };
    pub const BTN_MODE: EvCode = EvCode {
        page: super::PAGE_BUTTON,
        usage: super::USAGE_BTN_MODE,
    };
    pub const BTN_LTHUMB: EvCode = EvCode {
        page: super::PAGE_BUTTON,
        usage: super::USAGE_BTN_LTHUMB,
    };
    pub const BTN_RTHUMB: EvCode = EvCode {
        page: super::PAGE_BUTTON,
        usage: super::USAGE_BTN_RTHUMB,
    };

    pub const BTN_DPAD_UP: EvCode = EvCode {
        page: super::PAGE_BUTTON,
        usage: super::USAGE_BTN_DPAD_UP,
    };
    pub const BTN_DPAD_DOWN: EvCode = EvCode {
        page: super::PAGE_BUTTON,
        usage: super::USAGE_BTN_DPAD_DOWN,
    };
    pub const BTN_DPAD_LEFT: EvCode = EvCode {
        page: super::PAGE_BUTTON,
        usage: super::USAGE_BTN_DPAD_LEFT,
    };
    pub const BTN_DPAD_RIGHT: EvCode = EvCode {
        page: super::PAGE_BUTTON,
        usage: super::USAGE_BTN_DPAD_RIGHT,
    };
}
