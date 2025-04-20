// Copyright 2016-2018 Mateusz Sieczko and other GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
#![allow(non_upper_case_globals)]

use objc2_core_foundation::{
    kCFAllocatorDefault, CFArray, CFDictionary, CFNumber, CFRetained, CFString,
    CFStringBuiltInEncodings, CFType,
};
use objc2_io_kit::{
    io_service_t, kHIDPage_Button, kHIDPage_Consumer, kHIDPage_GenericDesktop, kHIDPage_Simulation,
    kHIDUsage_Button_1, kHIDUsage_GD_DPadDown, kHIDUsage_GD_DPadLeft, kHIDUsage_GD_DPadRight,
    kHIDUsage_GD_DPadUp, kHIDUsage_GD_Dial, kHIDUsage_GD_GamePad, kHIDUsage_GD_Hatswitch,
    kHIDUsage_GD_Joystick, kHIDUsage_GD_MultiAxisController, kHIDUsage_GD_Rx, kHIDUsage_GD_Ry,
    kHIDUsage_GD_Rz, kHIDUsage_GD_Select, kHIDUsage_GD_Slider, kHIDUsage_GD_Start,
    kHIDUsage_GD_SystemMainMenu, kHIDUsage_GD_Wheel, kHIDUsage_GD_X, kHIDUsage_GD_Y,
    kHIDUsage_GD_Z, kHIDUsage_Sim_Accelerator, kHIDUsage_Sim_Brake, kHIDUsage_Sim_Rudder,
    kHIDUsage_Sim_Throttle, kIOHIDDeviceUsageKey, kIOHIDDeviceUsagePageKey, kIOHIDLocationIDKey,
    kIOHIDOptionsTypeNone, kIOHIDPrimaryUsageKey, kIOHIDPrimaryUsagePageKey, kIOHIDProductIDKey,
    kIOHIDProductKey, kIOHIDVendorIDKey, kIOHIDVersionNumberKey, kIOReturnSuccess, IOHIDDevice,
    IOHIDElement, IOHIDElementType, IOHIDManager, IOObjectRelease, IOObjectRetain,
    IORegistryEntryGetRegistryEntryID, IO_OBJECT_NULL,
};

use std::ffi::CStr;

pub fn new_manager() -> Option<CFRetained<IOHIDManager>> {
    let manager = IOHIDManager::new(None, kIOHIDOptionsTypeNone);

    let matchers = CFArray::from_retained_objects(&[
        create_hid_device_matcher(kHIDPage_GenericDesktop, kHIDUsage_GD_Joystick),
        create_hid_device_matcher(kHIDPage_GenericDesktop, kHIDUsage_GD_GamePad),
        create_hid_device_matcher(kHIDPage_GenericDesktop, kHIDUsage_GD_MultiAxisController),
    ]);

    // SAFETY: The matchers are of the correct type.
    unsafe { manager.set_device_matching_multiple(Some(matchers.as_opaque())) };

    let ret = manager.open(kIOHIDOptionsTypeNone);
    if ret != kIOReturnSuccess {
        None
    } else {
        Some(manager)
    }
}

#[derive(Debug, Clone)]
pub struct Device(pub CFRetained<IOHIDDevice>);

// SAFETY: TODO, unsure?
unsafe impl Sync for Device {}
unsafe impl Send for Device {}

pub trait DeviceExt: Properties {
    fn device(&self) -> &IOHIDDevice;

    fn get_name(&self) -> Option<String> {
        self.get_string_property(kIOHIDProductKey)
            .map(|name| name.to_string())
    }

    fn get_location_id(&self) -> Option<u32> {
        self.get_number_property(kIOHIDLocationIDKey)
            .and_then(|location_id| location_id.as_i32().map(|location_id| location_id as u32))
    }

    fn get_vendor_id(&self) -> Option<u16> {
        self.get_number_property(kIOHIDVendorIDKey)
            .and_then(|vendor_id| vendor_id.as_i32().map(|vendor_id| vendor_id as u16))
    }

    fn get_product_id(&self) -> Option<u16> {
        self.get_number_property(kIOHIDProductIDKey)
            .and_then(|product_id| product_id.as_i32().map(|product_id| product_id as u16))
    }

    fn get_version(&self) -> Option<u16> {
        self.get_number_property(kIOHIDVersionNumberKey)
            .and_then(|version| version.as_i32().map(|version| version as u16))
    }

    fn get_page(&self) -> Option<u32> {
        self.get_number_property(kIOHIDPrimaryUsagePageKey)
            .and_then(|page| page.as_i32().map(|page| page as u32))
    }

    fn get_usage(&self) -> Option<u32> {
        self.get_number_property(kIOHIDPrimaryUsageKey)
            .and_then(|usage| usage.as_i32().map(|usage| usage as u32))
    }

    fn get_service(&self) -> Option<IOService> {
        IOService::new(self.device().service())
    }
}

pub fn device_elements(device: &IOHIDDevice) -> Vec<CFRetained<IOHIDElement>> {
    // SAFETY: We pass `None` as the dictionary, which means we don't have to worry about
    // type-safety there.
    let elements = unsafe { device.matching_elements(None, kIOHIDOptionsTypeNone) };

    let Some(elements) = elements else {
        return vec![];
    };

    // SAFETY: `IOHIDDeviceCopyMatchingElements` is documented to return CFArray of IOHIDElement.
    let elements = unsafe { elements.cast_unchecked::<IOHIDElement>() };

    elements.into_iter().collect()
}

impl DeviceExt for IOHIDDevice {
    fn device(&self) -> &IOHIDDevice {
        self
    }
}

impl Properties for IOHIDDevice {
    fn get_property(&self, key: &CStr) -> Option<CFRetained<CFType>> {
        debug_assert!(key.to_str().is_ok());
        // SAFETY: The key is a valid C string with UTF-8 contents.
        let key = unsafe {
            CFString::with_c_string(
                kCFAllocatorDefault,
                key.as_ptr(),
                CFStringBuiltInEncodings::EncodingUTF8.0,
            )?
        };
        self.property(&key)
    }
}

pub fn element_is_collection(type_: IOHIDElementType) -> bool {
    type_ == IOHIDElementType::Collection
}

pub fn element_is_axis(type_: IOHIDElementType, page: u32, usage: u32) -> bool {
    match type_ {
        IOHIDElementType::Input_Misc
        | IOHIDElementType::Input_Button
        | IOHIDElementType::Input_Axis => match page {
            kHIDPage_GenericDesktop => {
                matches!(
                    usage,
                    kHIDUsage_GD_X
                        | kHIDUsage_GD_Y
                        | kHIDUsage_GD_Z
                        | kHIDUsage_GD_Rx
                        | kHIDUsage_GD_Ry
                        | kHIDUsage_GD_Rz
                        | kHIDUsage_GD_Slider
                        | kHIDUsage_GD_Dial
                        | kHIDUsage_GD_Wheel
                )
            }
            kHIDPage_Simulation => matches!(
                usage,
                kHIDUsage_Sim_Rudder
                    | kHIDUsage_Sim_Throttle
                    | kHIDUsage_Sim_Accelerator
                    | kHIDUsage_Sim_Brake
            ),
            _ => false,
        },
        _ => false,
    }
}

pub fn element_is_button(type_: IOHIDElementType, page: u32, usage: u32) -> bool {
    match type_ {
        IOHIDElementType::Input_Misc
        | IOHIDElementType::Input_Button
        | IOHIDElementType::Input_Axis => match page {
            kHIDPage_GenericDesktop => matches!(
                usage,
                kHIDUsage_GD_DPadUp
                    | kHIDUsage_GD_DPadDown
                    | kHIDUsage_GD_DPadRight
                    | kHIDUsage_GD_DPadLeft
                    | kHIDUsage_GD_Start
                    | kHIDUsage_GD_Select
                    | kHIDUsage_GD_SystemMainMenu
            ),
            kHIDPage_Button | kHIDPage_Consumer => true,
            _ => false,
        },
        _ => false,
    }
}

pub fn element_is_hat(type_: IOHIDElementType, page: u32, usage: u32) -> bool {
    match type_ {
        IOHIDElementType::Input_Misc
        | IOHIDElementType::Input_Button
        | IOHIDElementType::Input_Axis => match page {
            kHIDPage_GenericDesktop => matches!(usage, USAGE_AXIS_DPADX | USAGE_AXIS_DPADY),
            _ => false,
        },
        _ => false,
    }
}

pub fn element_children(element: &IOHIDElement) -> Vec<CFRetained<IOHIDElement>> {
    let elements = element.children();

    let Some(elements) = elements else {
        return vec![];
    };

    // SAFETY: `IOHIDElementGetChildren` is documented to return CFArray of IOHIDElement.
    let elements = unsafe { elements.cast_unchecked::<IOHIDElement>() };

    elements.into_iter().collect()
}

impl Properties for IOHIDElement {
    fn get_property(&self, key: &CStr) -> Option<CFRetained<CFType>> {
        debug_assert!(key.to_str().is_ok());
        // SAFETY: The key is a valid C string with UTF-8 contents.
        let key = unsafe {
            CFString::with_c_string(
                kCFAllocatorDefault,
                key.as_ptr(),
                CFStringBuiltInEncodings::EncodingUTF8.0,
            )?
        };
        self.property(&key)
    }
}

#[repr(C)]
#[derive(Debug)]
pub(crate) struct IOService(io_service_t);

impl IOService {
    pub fn new(io_service: io_service_t) -> Option<IOService> {
        if io_service == IO_OBJECT_NULL {
            return None;
        }

        // We pair this retain with a release in `Drop`.
        let result = IOObjectRetain(io_service);

        if result == kIOReturnSuccess {
            Some(IOService(io_service))
        } else {
            None
        }
    }

    pub fn get_registry_entry_id(&self) -> Option<u64> {
        IOObjectRetain(self.0);

        let mut entry_id = 0;
        // SAFETY: `&mut entry_id` is a valid pointer.
        let result = unsafe { IORegistryEntryGetRegistryEntryID(self.0, &mut entry_id) };

        IOObjectRelease(self.0);

        if result == kIOReturnSuccess {
            Some(entry_id)
        } else {
            None
        }
    }
}

impl Drop for IOService {
    fn drop(&mut self) {
        IOObjectRelease(self.0 as _);
    }
}

pub trait Properties {
    fn get_property(&self, key: &CStr) -> Option<CFRetained<CFType>>;

    fn get_number_property(&self, key: &CStr) -> Option<CFRetained<CFNumber>> {
        self.get_property(key)
            .and_then(|value| value.downcast::<CFNumber>().ok())
    }

    fn get_string_property(&self, key: &CStr) -> Option<CFRetained<CFString>> {
        self.get_property(key)
            .and_then(|value| value.downcast::<CFString>().ok())
    }
}

fn create_hid_device_matcher(
    page: u32,
    usage: u32,
) -> CFRetained<CFDictionary<CFString, CFNumber>> {
    let page_key = CFString::from_static_str(kIOHIDDeviceUsagePageKey.to_str().unwrap());
    let page_value = CFNumber::new_i32(page as i32);

    let usage_key = CFString::from_static_str(kIOHIDDeviceUsageKey.to_str().unwrap());
    let usage_value = CFNumber::new_i32(usage as i32);

    CFDictionary::from_slices(&[&*page_key, &*usage_key], &[&*page_value, &*usage_value])
}

// Revisions:
// - MacOS Version: Sequoia 15.5 (Xbox One Elite Series 2 Controller) (20th of July 2025)

// Usage Pages
pub const PAGE_GENERIC_DESKTOP: u32 = kHIDPage_GenericDesktop;
pub const PAGE_SIMULATION: u32 = kHIDPage_Simulation;
pub const PAGE_BUTTON: u32 = kHIDPage_Button;

// GenericDesktop Page (0x01)
pub const USAGE_AXIS_LSTICKX: u32 = kHIDUsage_GD_X;
pub const USAGE_AXIS_LSTICKY: u32 = kHIDUsage_GD_Y;
pub const USAGE_AXIS_LEFTZ: u32 = 0; // unconfirmed
pub const USAGE_AXIS_RSTICKX: u32 = kHIDUsage_GD_Z;
pub const USAGE_AXIS_RSTICKY: u32 = kHIDUsage_GD_Rz;
pub const USAGE_AXIS_RIGHTZ: u32 = 0; // unconfirmed
pub const USAGE_AXIS_DPADX: u32 = kHIDUsage_GD_Hatswitch;
pub const USAGE_AXIS_DPADY: u32 = kHIDUsage_GD_Hatswitch + 1;
pub const USAGE_AXIS_RT: u32 = 0; // unconfirmed
pub const USAGE_AXIS_LT: u32 = 0; // unconfirmed
pub const USAGE_AXIS_RT2: u32 = kHIDUsage_Sim_Accelerator;
pub const USAGE_AXIS_LT2: u32 = kHIDUsage_Sim_Brake;

// Button Page (0x09)
pub const USAGE_BTN_SOUTH: u32 = kHIDUsage_Button_1;
pub const USAGE_BTN_EAST: u32 = kHIDUsage_Button_1 + 1;
pub const USAGE_BTN_WEST: u32 = kHIDUsage_Button_1 + 3;
pub const USAGE_BTN_NORTH: u32 = kHIDUsage_Button_1 + 4;
pub const USAGE_BTN_LT: u32 = kHIDUsage_Button_1 + 6;
pub const USAGE_BTN_RT: u32 = kHIDUsage_Button_1 + 7;
pub const USAGE_BTN_LT2: u32 = kHIDUsage_Button_1 + 8; // unconfirmed
pub const USAGE_BTN_RT2: u32 = kHIDUsage_Button_1 + 9; // unconfirmed
pub const USAGE_BTN_SELECT: u32 = kHIDUsage_Button_1 + 10;
pub const USAGE_BTN_START: u32 = kHIDUsage_Button_1 + 11;
pub const USAGE_BTN_MODE: u32 = kHIDUsage_Button_1 + 12;
pub const USAGE_BTN_LTHUMB: u32 = kHIDUsage_Button_1 + 13;
pub const USAGE_BTN_RTHUMB: u32 = kHIDUsage_Button_1 + 14;
pub const USAGE_BTN_DPAD_UP: u32 = kHIDUsage_Button_1 + 15; // unconfirmed
pub const USAGE_BTN_DPAD_DOWN: u32 = kHIDUsage_Button_1 + 16; // unconfirmed
pub const USAGE_BTN_DPAD_LEFT: u32 = kHIDUsage_Button_1 + 17; // unconfirmed
pub const USAGE_BTN_DPAD_RIGHT: u32 = kHIDUsage_Button_1 + 18; // unconfirmed
pub const USAGE_BTN_C: u32 = kHIDUsage_Button_1 + 19; // unconfirmed
pub const USAGE_BTN_Z: u32 = kHIDUsage_Button_1 + 20; // unconfirmed
