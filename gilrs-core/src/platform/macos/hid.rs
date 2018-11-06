// Copyright 2016-2018 Mateusz Sieczko and other GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]

use core_foundation::array::CFArray;
use core_foundation::base::{kCFAllocatorDefault, CFRelease, TCFType};
use core_foundation::dictionary::CFDictionary;
use core_foundation::number::CFNumber;
use core_foundation::runloop::{CFRunLoop, CFRunLoopMode};
use core_foundation::string::CFString;

use io_kit_sys::hid::base::{IOHIDDeviceCallback, IOHIDValueCallback};
use io_kit_sys::hid::keys::*;
use io_kit_sys::hid::manager::*;
use io_kit_sys::hid::usage_tables::*;
use io_kit_sys::ret::{kIOReturnSuccess, IOReturn};

use std::ffi::CStr;
use std::os::raw::c_void;

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
}

impl Drop for IOHIDManager {
    fn drop(&mut self) {
        unsafe { CFRelease(self.as_CFTypeRef()) }
    }
}

unsafe impl Send for IOHIDManager {}
unsafe impl Sync for IOHIDManager {}

fn create_hid_device_matcher(page: u32, usage: u32) -> CFDictionary<CFString, CFNumber> {
    let page_key = unsafe { CStr::from_ptr(kIOHIDDeviceUsagePageKey as _) };
    let page_key = CFString::from(page_key.to_str().unwrap());
    let page_value = CFNumber::from(page as i32);

    let usage_key = unsafe { CStr::from_ptr(kIOHIDDeviceUsageKey as _) };
    let usage_key = CFString::from(usage_key.to_str().unwrap());
    let usage_value = CFNumber::from(usage as i32);

    CFDictionary::from_CFType_pairs(&[(page_key, page_value), (usage_key, usage_value)])
}
