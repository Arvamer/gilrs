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

use io_kit_sys::hid::usage_tables::{
    kHIDPage_GenericDesktop, kHIDUsage_GD_GamePad, kHIDUsage_GD_Joystick,
    kHIDUsage_GD_MultiAxisController,
};
use vec_map::VecMap;

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
    name: String,
    uuid: Uuid,
    location_id: u32,
    page: u32,
    usage: u32,
    axes_info: VecMap<AxisInfo>,
    axes: Vec<EvCode>,
    buttons: Vec<EvCode>,
    is_connected: bool,
}

impl Gamepad {
    fn open(device: IOHIDDevice) -> Option<Gamepad> {
        let location_id = match device.get_location_id() {
            Some(location_id) => location_id,
            None => {
                error!("Failed to get location id of device");
                return None;
            }
        };

        let page = match device.get_page() {
            Some(page) => if page == kHIDPage_GenericDesktop {
                page
            } else {
                error!("Failed to get valid device: {:?}", page);
                return None;
            },
            None => {
                error!("Failed to get page of device");
                return None;
            }
        };

        let usage = match device.get_usage() {
            Some(usage) => if usage == kHIDUsage_GD_GamePad
                || usage == kHIDUsage_GD_Joystick
                || usage == kHIDUsage_GD_MultiAxisController
            {
                usage
            } else {
                error!("Failed to get valid device: {:?}", usage);
                return None;
            },
            None => {
                error!("Failed to get usage of device");
                return None;
            }
        };

        let name = device.get_name().unwrap_or_else(|| {
            warn!("Failed to get name of device");
            "Unknown".into()
        });

        let uuid = match Self::create_uuid(&device) {
            Some(uuid) => uuid,
            None => {
                return None;
            }
        };

        let mut gamepad = Gamepad {
            name: name,
            uuid: uuid,
            location_id: location_id,
            page: page,
            usage: usage,
            axes_info: VecMap::with_capacity(8),
            axes: Vec::with_capacity(8),
            buttons: Vec::with_capacity(16),
            is_connected: true,
        };

        let elements = device.get_elements();
        let mut axes = VecMap::with_capacity(8);
        Self::find_axes(&elements, &mut axes);

        for (_, axis) in axes.iter_mut() {
            let ev_code = axis.ev_code;
            gamepad.axes_info.insert(ev_code.usage as usize, axis.info);
            gamepad.axes.push(ev_code);
        }

        let mut buttons = VecMap::with_capacity(16);

        Self::find_buttons(&elements, &mut buttons);

        for (_, button) in buttons.iter_mut() {
            gamepad.buttons.push(*button);
        }

        Some(gamepad)
    }

    fn create_uuid(device: &IOHIDDevice) -> Option<Uuid> {
        let bustype = match device.get_bustype() {
            Some(bustype) => (bustype as u32).to_be(),
            None => {
                warn!("Failed to get transport key of device");
                0
            }
        };

        let vendor_id = match device.get_vendor_id() {
            Some(vendor_id) => vendor_id.to_be(),
            None => {
                warn!("Failed to get vendor id of device");
                0
            }
        };

        let product_id = match device.get_product_id() {
            Some(product_id) => product_id.to_be(),
            None => {
                warn!("Failed to get product id of device");
                0
            }
        };

        let version = match device.get_version() {
            Some(version) => version.to_be(),
            None => {
                warn!("Failed to get version of device");
                0
            }
        };

        if vendor_id == 0 && product_id == 0 && version == 0 {
            None
        } else {
            match Uuid::from_fields(
                bustype,
                vendor_id,
                0,
                &[
                    (product_id >> 8) as u8,
                    product_id as u8,
                    0,
                    0,
                    (version >> 8) as u8,
                    version as u8,
                    0,
                    0,
                ],
            ) {
                Ok(uuid) => Some(uuid),
                Err(error) => {
                    error!("Failed to create uuid of device: {:?}", error.to_string());
                    None
                }
            }
        }
    }

    fn find_axes(elements: &Vec<IOHIDElement>, axes: &mut VecMap<Axis>) {
        for element in elements {
            let type_ = element.get_type();
            let cookie = element.get_cookie();
            let page = element.get_page();
            let usage = element.get_usage();

            if IOHIDElement::is_collection_type(type_) {
                let children = element.get_children();
                Self::find_axes(&children, axes);
            } else if IOHIDElement::is_axis(type_, page, usage) {
                axes.insert(
                    cookie as usize,
                    Axis {
                        ev_code: EvCode::new(page, usage),
                        info: AxisInfo {
                            min: element.get_logical_min() as _,
                            max: element.get_logical_max() as _,
                            deadzone: 0,
                        },
                    },
                );
            }
        }
    }

    fn find_buttons(elements: &Vec<IOHIDElement>, buttons: &mut VecMap<EvCode>) {
        for element in elements {
            let type_ = element.get_type();
            let cookie = element.get_cookie();
            let page = element.get_page();
            let usage = element.get_usage();

            if IOHIDElement::is_collection_type(type_) {
                let children = element.get_children();
                Self::find_buttons(&children, buttons);
            } else if IOHIDElement::is_button(type_, page, usage) {
                buttons.insert(cookie as usize, EvCode::new(page, usage));
            }
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn uuid(&self) -> Uuid {
        self.uuid
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
        &self.buttons
    }

    pub fn axes(&self) -> &[EvCode] {
        &self.axes
    }

    pub(crate) fn axis_info(&self, nec: EvCode) -> Option<&AxisInfo> {
        self.axes_info.get(nec.usage as usize)
    }

    pub fn is_connected(&self) -> bool {
        self.is_connected
    }
}

#[derive(Debug)]
struct Axis {
    ev_code: EvCode,
    info: AxisInfo,
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
