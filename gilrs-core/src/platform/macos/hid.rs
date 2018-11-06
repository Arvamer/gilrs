// Copyright 2016-2018 Mateusz Sieczko and other GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]

use core_foundation::array::{
    kCFTypeArrayCallBacks, CFArray, CFArrayAppendValue, CFArrayCreateMutable, CFArrayGetCount,
    CFArrayGetValueAtIndex,
};
use core_foundation::base::{kCFAllocatorDefault, CFRelease, CFType, TCFType};
use core_foundation::dictionary::CFDictionary;
use core_foundation::number::CFNumber;
use core_foundation::runloop::{CFRunLoop, CFRunLoopMode};
use core_foundation::set::CFSetApplyFunction;
use core_foundation::string::{kCFStringEncodingUTF8, CFString, CFStringCreateWithCString};

use io_kit_sys::hid::base::{IOHIDDeviceCallback, IOHIDDeviceRef, IOHIDValueCallback};
use io_kit_sys::hid::device::{IOHIDDeviceGetProperty, IOHIDDeviceGetTypeID};
use io_kit_sys::hid::keys::*;
use io_kit_sys::hid::manager::*;
use io_kit_sys::hid::usage_tables::*;
use io_kit_sys::ret::{kIOReturnSuccess, IOReturn};

use std::ffi::CStr;
use std::os::raw::{c_char, c_void};

#[repr(C)]
#[derive(Debug)]
pub struct IOHIDManager(IOHIDManagerRef);

impl_TCFType!(IOHIDManager, IOHIDManagerRef, IOHIDManagerGetTypeID);

impl IOHIDManager {
    pub fn new() -> Option<Self> {
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

    pub fn open(&mut self) -> IOReturn {
        unsafe { IOHIDManagerOpen(self.0, kIOHIDOptionsTypeNone) }
    }

    pub fn close(&mut self) -> IOReturn {
        unsafe { IOHIDManagerClose(self.0, kIOHIDOptionsTypeNone) }
    }

    pub fn schedule_with_run_loop(&mut self, run_loop: CFRunLoop, run_loop_mode: CFRunLoopMode) {
        unsafe {
            IOHIDManagerScheduleWithRunLoop(self.0, run_loop.as_concrete_TypeRef(), run_loop_mode)
        }
    }

    pub fn unschedule_from_run_loop(&mut self, run_loop: CFRunLoop, run_loop_mode: CFRunLoopMode) {
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

    pub fn get_devices(&mut self) -> Vec<IOHIDDevice> {
        let copied = unsafe { IOHIDManagerCopyDevices(self.0) };

        if copied.is_null() {
            return vec![];
        }

        let devices =
            unsafe { CFArrayCreateMutable(kCFAllocatorDefault, 0, &kCFTypeArrayCallBacks) };

        if devices.is_null() {
            unsafe { CFRelease(copied as _) };
            return vec![];
        }

        unsafe { CFSetApplyFunction(copied, cf_set_applier, devices as _) };
        unsafe { CFRelease(copied as _) };

        let device_count = unsafe { CFArrayGetCount(devices) };
        let mut vec = Vec::with_capacity(device_count as _);

        for i in 0..device_count {
            let device = unsafe { CFArrayGetValueAtIndex(devices, i) };

            if device.is_null() {
                continue;
            }

            if let Some(device) = IOHIDDevice::new(device as _) {
                vec.push(device);
            }
        }

        unsafe { CFRelease(devices as _) };

        vec
    }
}

impl Drop for IOHIDManager {
    fn drop(&mut self) {
        unsafe { CFRelease(self.as_CFTypeRef()) }
    }
}

unsafe impl Send for IOHIDManager {}
unsafe impl Sync for IOHIDManager {}

#[repr(C)]
#[derive(Debug)]
pub struct IOHIDDevice(IOHIDDeviceRef);

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
        match self.get_string_property(kIOHIDProductKey) {
            Some(name) => Some(name.to_string()),
            None => None,
        }
    }

    pub fn get_location_id(&self) -> Option<u32> {
        match self.get_number_property(kIOHIDLocationIDKey) {
            Some(location_id) => match location_id.to_i32() {
                Some(location_id) => Some(location_id as u32),
                None => None,
            },
            None => None,
        }
    }

    pub fn get_bustype(&self) -> Option<u16> {
        match self.get_transport_key() {
            Some(transport_key) => {
                if transport_key == "USB".to_string() {
                    Some(0x03)
                } else if transport_key == "Bluetooth".to_string() {
                    Some(0x05)
                } else {
                    None
                }
            }
            None => None,
        }
    }

    pub fn get_transport_key(&self) -> Option<String> {
        match self.get_string_property(kIOHIDTransportKey) {
            Some(transport_key) => Some(transport_key.to_string()),
            None => None,
        }
    }

    pub fn get_vendor_id(&self) -> Option<u16> {
        match self.get_number_property(kIOHIDVendorIDKey) {
            Some(vendor_id) => match vendor_id.to_i32() {
                Some(vendor_id) => Some(vendor_id as u16),
                None => None,
            },
            None => None,
        }
    }

    pub fn get_product_id(&self) -> Option<u16> {
        match self.get_number_property(kIOHIDProductIDKey) {
            Some(product_id) => match product_id.to_i32() {
                Some(product_id) => Some(product_id as u16),
                None => None,
            },
            None => None,
        }
    }

    pub fn get_version(&self) -> Option<u16> {
        match self.get_number_property(kIOHIDVersionNumberKey) {
            Some(version) => match version.to_i32() {
                Some(version) => Some(version as u16),
                None => None,
            },
            None => None,
        }
    }

    pub fn get_page(&self) -> Option<u32> {
        match self.get_number_property(kIOHIDPrimaryUsagePageKey) {
            Some(page) => match page.to_i32() {
                Some(page) => Some(page as u32),
                None => None,
            },
            None => None,
        }
    }

    pub fn get_usage(&self) -> Option<u32> {
        match self.get_number_property(kIOHIDPrimaryUsageKey) {
            Some(usage) => match usage.to_i32() {
                Some(usage) => Some(usage as u32),
                None => None,
            },
            None => None,
        }
    }
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

trait Properties {
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

extern "C" fn cf_set_applier(value: *const c_void, context: *const c_void) {
    unsafe { CFArrayAppendValue(context as _, value) };
}
