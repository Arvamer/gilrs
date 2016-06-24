extern crate gamepad;
use gamepad::udev::*;
use std::ffi::{CString, CStr};
use std::os::raw::c_char;

fn is_gamepad(dev: &Device) -> bool {
    let mut is_gamepad = false;
    let mut is_keyboard = false;
    for (key, _) in dev.properties() {
        if key == "ID_INPUT_JOYSTICK" {
            is_gamepad = true;
        } else if key == "ID_INPUT_KEYBOARD" {
            is_keyboard = true;
        }
    }
    if is_gamepad && !is_keyboard {
        true
    } else {
        false
    }
}

extern "C" {
    fn strstr(s: *const c_char, sub: *const c_char) -> *mut c_char;
}

fn is_event(path: &CStr) -> bool {
    unsafe {
        if strstr(path.as_ptr(), b"/event\0".as_ptr() as *const i8).is_null() {
            false
        } else {
            true
        }
    }
}

fn main() {
    let udev = Udev::new().unwrap();
    let en = udev.enumerate().unwrap();
    en.add_match_subsytem(&CString::new("input").unwrap());
    en.scan_devices();
    let it = en.iter();
    for d in it {
        if !is_event(&d) {
            continue;
        }
        let dev = Device::from_syspath(&udev, &d).unwrap();
        if is_gamepad(&dev) {
            println!("{:?}: ", d);
            for (key, val) in dev.properties() {
                println!("    {} = {}", key, val);
            }
        }
    }
}
