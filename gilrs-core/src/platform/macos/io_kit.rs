// Copyright 2016-2018 Mateusz Sieczko and other GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]

use core_foundation::array::{CFArray, CFArrayGetCount, CFArrayGetValueAtIndex};
use core_foundation::base::{kCFAllocatorDefault, CFRelease, CFType, TCFType};
use core_foundation::dictionary::CFDictionary;
use core_foundation::impl_TCFType;
use core_foundation::number::CFNumber;
use core_foundation::runloop::{CFRunLoop, CFRunLoopMode};
use core_foundation::string::{kCFStringEncodingUTF8, CFString, CFStringCreateWithCString};

use io_kit_sys::hid::base::{
    IOHIDDeviceCallback, IOHIDDeviceRef, IOHIDElementRef, IOHIDValueCallback, IOHIDValueRef,
};
use io_kit_sys::hid::device::*;
use io_kit_sys::hid::element::*;
use io_kit_sys::hid::keys::*;
use io_kit_sys::hid::manager::*;
use io_kit_sys::hid::usage_tables::*;
use io_kit_sys::hid::value::{
    IOHIDValueGetElement, IOHIDValueGetIntegerValue, IOHIDValueGetTypeID,
};
use io_kit_sys::ret::kIOReturnSuccess;
use io_kit_sys::types::{io_service_t, IO_OBJECT_NULL};
use io_kit_sys::{IOObjectRelease, IOObjectRetain, IORegistryEntryGetRegistryEntryID};

use std::ffi::CStr;
use std::os::raw::{c_char, c_void};
use std::ptr;

#[repr(C)]
#[derive(Debug)]
pub(crate) struct IOHIDManager(IOHIDManagerRef);

impl_TCFType!(IOHIDManager, IOHIDManagerRef, IOHIDManagerGetTypeID);

pub fn new_manager() -> Option<IOHIDManager> {
    let manager = unsafe { IOHIDManagerCreate(kCFAllocatorDefault, kIOHIDOptionsTypeNone) };

    if manager.is_null() {
        return None;
    }

    let matchers = CFArray::from_CFTypes(&[
        create_hid_device_matcher(kHIDPage_GenericDesktop, kHIDUsage_GD_Joystick),
        create_hid_device_matcher(kHIDPage_GenericDesktop, kHIDUsage_GD_GamePad),
        create_hid_device_matcher(kHIDPage_GenericDesktop, kHIDUsage_GD_MultiAxisController),
    ]);
    unsafe {
        IOHIDManagerSetDeviceMatchingMultiple(manager, matchers.as_concrete_TypeRef());
    };

    let ret = unsafe { IOHIDManagerOpen(manager, kIOHIDOptionsTypeNone) };

    if ret == kIOReturnSuccess {
        Some(IOHIDManager(manager))
    } else {
        unsafe { CFRelease(manager as _) };
        None
    }
}

impl IOHIDManager {
    pub fn schedule_with_run_loop(&mut self, run_loop: &CFRunLoop, run_loop_mode: CFRunLoopMode) {
        unsafe {
            IOHIDManagerScheduleWithRunLoop(self.0, run_loop.as_concrete_TypeRef(), run_loop_mode)
        }
    }

    pub fn unschedule_from_run_loop(&mut self, run_loop: &CFRunLoop, run_loop_mode: CFRunLoopMode) {
        unsafe {
            IOHIDManagerUnscheduleFromRunLoop(self.0, run_loop.as_concrete_TypeRef(), run_loop_mode)
        }
    }

    pub fn register_device_matching_callback(
        &mut self,
        callback: IOHIDDeviceCallback,
        context: *mut c_void,
    ) {
        unsafe { IOHIDManagerRegisterDeviceMatchingCallback(self.0, callback, context) }
    }

    pub fn register_device_removal_callback(
        &mut self,
        callback: IOHIDDeviceCallback,
        context: *mut c_void,
    ) {
        unsafe { IOHIDManagerRegisterDeviceRemovalCallback(self.0, callback, context) }
    }

    pub fn register_input_value_callback(
        &mut self,
        callback: IOHIDValueCallback,
        context: *mut c_void,
    ) {
        unsafe { IOHIDManagerRegisterInputValueCallback(self.0, callback, context) }
    }
}

impl Drop for IOHIDManager {
    fn drop(&mut self) {
        unsafe { CFRelease(self.as_CFTypeRef()) }
    }
}

#[repr(C)]
#[derive(Debug)]
pub(crate) struct IOHIDDevice(IOHIDDeviceRef);

impl_TCFType!(IOHIDDevice, IOHIDDeviceRef, IOHIDDeviceGetTypeID);

impl IOHIDDevice {
    pub fn new(device: IOHIDDeviceRef) -> Option<IOHIDDevice> {
        if device.is_null() {
            None
        } else {
            Some(IOHIDDevice(device))
        }
    }

    pub fn get_name(&self) -> Option<String> {
        self.get_string_property(kIOHIDProductKey)
            .map(|name| name.to_string())
    }

    pub fn get_location_id(&self) -> Option<u32> {
        self.get_number_property(kIOHIDLocationIDKey)
            .and_then(|location_id| location_id.to_i32().map(|location_id| location_id as u32))
    }

    pub fn get_vendor_id(&self) -> Option<u16> {
        self.get_number_property(kIOHIDVendorIDKey)
            .and_then(|vendor_id| vendor_id.to_i32().map(|vendor_id| vendor_id as u16))
    }

    pub fn get_product_id(&self) -> Option<u16> {
        self.get_number_property(kIOHIDProductIDKey)
            .and_then(|product_id| product_id.to_i32().map(|product_id| product_id as u16))
    }

    pub fn get_version(&self) -> Option<u16> {
        self.get_number_property(kIOHIDVersionNumberKey)
            .and_then(|version| version.to_i32().map(|version| version as u16))
    }

    pub fn get_page(&self) -> Option<u32> {
        self.get_number_property(kIOHIDPrimaryUsagePageKey)
            .and_then(|page| page.to_i32().map(|page| page as u32))
    }

    pub fn get_usage(&self) -> Option<u32> {
        self.get_number_property(kIOHIDPrimaryUsageKey)
            .and_then(|usage| usage.to_i32().map(|usage| usage as u32))
    }

    pub fn get_service(&self) -> Option<IOService> {
        unsafe { IOService::new(IOHIDDeviceGetService(self.0)) }
    }
}

pub fn device_elements(device: &IOHIDDevice) -> Vec<IOHIDElement> {
    let elements =
        unsafe { IOHIDDeviceCopyMatchingElements(device.0, ptr::null(), kIOHIDOptionsTypeNone) };

    if elements.is_null() {
        return vec![];
    }

    let element_count = unsafe { CFArrayGetCount(elements) };
    let mut vec = Vec::with_capacity(element_count as _);

    for i in 0..element_count {
        let element = unsafe { CFArrayGetValueAtIndex(elements, i) };

        if element.is_null() {
            continue;
        }

        vec.push(IOHIDElement(element as _));
    }

    vec
}

impl Properties for IOHIDDevice {
    fn get_property(&self, key: *const c_char) -> Option<CFType> {
        let key =
            unsafe { CFStringCreateWithCString(kCFAllocatorDefault, key, kCFStringEncodingUTF8) };
        let value = unsafe { IOHIDDeviceGetProperty(self.0, key) };

        if value.is_null() {
            None
        } else {
            Some(unsafe { TCFType::wrap_under_get_rule(value) })
        }
    }
}

unsafe impl Send for IOHIDDevice {}
unsafe impl Sync for IOHIDDevice {}

#[repr(C)]
#[derive(Debug)]
pub(crate) struct IOHIDElement(IOHIDElementRef);

impl_TCFType!(IOHIDElement, IOHIDElementRef, IOHIDElementGetTypeID);

pub fn element_is_collection(type_: IOHIDElementType) -> bool {
    type_ == kIOHIDElementTypeCollection
}

pub fn element_is_axis(type_: IOHIDElementType, page: u32, usage: u32) -> bool {
    match type_ {
        kIOHIDElementTypeInput_Misc
        | kIOHIDElementTypeInput_Button
        | kIOHIDElementTypeInput_Axis => match page {
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
        kIOHIDElementTypeInput_Misc
        | kIOHIDElementTypeInput_Button
        | kIOHIDElementTypeInput_Axis => match page {
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
        kIOHIDElementTypeInput_Misc
        | kIOHIDElementTypeInput_Button
        | kIOHIDElementTypeInput_Axis => match page {
            kHIDPage_GenericDesktop => matches!(usage, USAGE_AXIS_DPADX | USAGE_AXIS_DPADY),
            _ => false,
        },
        _ => false,
    }
}

impl IOHIDElement {
    pub fn cookie(&self) -> u32 {
        unsafe { IOHIDElementGetCookie(self.0) }
    }

    pub fn r#type(&self) -> u32 {
        unsafe { IOHIDElementGetType(self.0) }
    }

    pub fn usage_page(&self) -> u32 {
        unsafe { IOHIDElementGetUsagePage(self.0) }
    }

    pub fn usage(&self) -> u32 {
        unsafe { IOHIDElementGetUsage(self.0) }
    }

    pub fn logical_min(&self) -> isize {
        unsafe { IOHIDElementGetLogicalMin(self.0) }
    }

    pub fn logical_max(&self) -> isize {
        unsafe { IOHIDElementGetLogicalMax(self.0) }
    }
}

pub fn element_children(element: &IOHIDElement) -> Vec<IOHIDElement> {
    let elements = unsafe { IOHIDElementGetChildren(element.0) };

    if elements.is_null() {
        return vec![];
    }

    let element_count = unsafe { CFArrayGetCount(elements) };
    let mut vec = Vec::with_capacity(element_count as _);

    for i in 0..element_count {
        let element = unsafe { CFArrayGetValueAtIndex(elements, i) };

        if element.is_null() {
            continue;
        }

        vec.push(IOHIDElement(element as _));
    }

    vec
}

impl Properties for IOHIDElement {
    fn get_property(&self, key: *const c_char) -> Option<CFType> {
        let key =
            unsafe { CFStringCreateWithCString(kCFAllocatorDefault, key, kCFStringEncodingUTF8) };
        let value = unsafe { IOHIDElementGetProperty(self.0, key) };

        if value.is_null() {
            None
        } else {
            Some(unsafe { TCFType::wrap_under_get_rule(value) })
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub(crate) struct IOHIDValue(IOHIDValueRef);

impl_TCFType!(IOHIDValue, IOHIDValueRef, IOHIDValueGetTypeID);

impl IOHIDValue {
    pub fn new(value: IOHIDValueRef) -> Option<IOHIDValue> {
        if value.is_null() {
            None
        } else {
            Some(IOHIDValue(value))
        }
    }

    pub fn integer_value(&self) -> isize {
        unsafe { IOHIDValueGetIntegerValue(self.0) }
    }

    pub fn element(&self) -> Option<IOHIDElement> {
        let element = unsafe { IOHIDValueGetElement(self.0) };

        if element.is_null() {
            None
        } else {
            Some(IOHIDElement(element))
        }
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

        let result = unsafe { IOObjectRetain(io_service) };

        if result == kIOReturnSuccess {
            Some(IOService(io_service))
        } else {
            None
        }
    }

    pub fn get_registry_entry_id(&self) -> Option<u64> {
        unsafe {
            IOObjectRetain(self.0);

            let mut entry_id = 0;
            let result = IORegistryEntryGetRegistryEntryID(self.0, &mut entry_id);

            IOObjectRelease(self.0);

            if result == kIOReturnSuccess {
                Some(entry_id)
            } else {
                None
            }
        }
    }
}

impl Drop for IOService {
    fn drop(&mut self) {
        unsafe {
            IOObjectRelease(self.0 as _);
        }
    }
}

trait Properties {
    fn get_number_property(&self, key: *const c_char) -> Option<CFNumber> {
        match self.get_property(key) {
            Some(value) => {
                if value.instance_of::<CFNumber>() {
                    Some(unsafe { CFNumber::wrap_under_get_rule(value.as_CFTypeRef() as _) })
                } else {
                    None
                }
            }
            None => None,
        }
    }

    fn get_string_property(&self, key: *const c_char) -> Option<CFString> {
        match self.get_property(key) {
            Some(value) => {
                if value.instance_of::<CFString>() {
                    Some(unsafe { CFString::wrap_under_get_rule(value.as_CFTypeRef() as _) })
                } else {
                    None
                }
            }
            None => None,
        }
    }

    fn get_property(&self, key: *const c_char) -> Option<CFType>;
}

fn create_hid_device_matcher(page: u32, usage: u32) -> CFDictionary<CFString, CFNumber> {
    let page_key = unsafe { CStr::from_ptr(kIOHIDDeviceUsagePageKey as _) };
    let page_key = CFString::from(page_key.to_str().unwrap());
    let page_value = CFNumber::from(page as i32);

    let usage_key = unsafe { CStr::from_ptr(kIOHIDDeviceUsageKey as _) };
    let usage_key = CFString::from(usage_key.to_str().unwrap());
    let usage_value = CFNumber::from(usage as i32);

    CFDictionary::from_CFType_pairs(&[(page_key, page_value), (usage_key, usage_value)])
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
