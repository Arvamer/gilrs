extern crate gamepad;
extern crate libc;
#[macro_use] extern crate ioctl;

use gamepad::udev::*;
use std::ffi::{CString, CStr};
use std::mem;
use std::fs::File;
use std::path::Path;
use std::io::prelude::*;
use libc as c;

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

fn is_event(path: &CStr) -> bool {
    unsafe {
        if c::strstr(path.as_ptr(), b"/event\0".as_ptr() as *const i8).is_null() {
            false
        } else {
            true
        }
    }
}

/*fn nbits(x: u32) {
    ((x - 1) / mem::size_of::<usize>() * 8) + 1
}*/

const KEY_MAX: u32 = 0x2ff;
const EV_MAX: u32 = 0x1f;
const ABS_MAX: u32 = 0x3f;

const EV_KEY: u32 = 0x01;
const EV_ABS: u32 = 0x03;
const EV_FF: u32 = 0x15;

fn test_bit(n: u32, array: &[u8]) -> bool {
    (array[(n / 8) as usize] >> (n % 8)) & 1 != 0
}

fn events<P: AsRef<Path>>(path: P) {
    let mut file = File::open(path).unwrap();
    let mut buff = [0; 24];
    loop {
        let _ = file.read_exact(&mut buff);
        let event = unsafe { mem::transmute::<_, ioctl::input_event>(buff) };
        println!("{:?}", event);
    }
}

fn main() {
    let udev = Udev::new().unwrap();
    let en = udev.enumerate().unwrap();
    en.add_match_subsytem(&CString::new("input").unwrap());
    en.scan_devices();
    let it = en.iter();

    let mut path = CString::new("").unwrap();

    for d in it {
        if !is_event(&d) {
            continue;
        }
        let dev = Device::from_syspath(&udev, &d).unwrap();
        if is_gamepad(&dev) {
            println!("{:?}: ", d);
        }
    }

    unsafe {
        let fd = c::open(b"/dev/input/event18\0".as_ptr() as *const i8, c::O_RDWR);
        if fd < 0 {
            panic!(format!("Failed to open {:?}", path));
        }

        let mut evbit = [0; 32];
        let mut keybit = [0; 32*8];
        let mut ffbit = [0; 32*8];

        let ev = ioctl::eviocgbit(fd, 0, 32, evbit.as_mut_ptr());
        let key = ioctl::eviocgbit(fd, EV_KEY, 32*8, keybit.as_mut_ptr());
        let abs = ioctl::eviocgbit(fd, EV_FF, 32*8, ffbit.as_mut_ptr());

        println!("Keys: {}, Axis: {}, Led: {}", test_bit(EV_KEY, &evbit), test_bit(EV_ABS, &evbit), test_bit(11, &evbit));
        println!("Gamepad: {}", test_bit(0x130, &keybit));
        c::close(fd);

        events("/dev/input/event18");
    }
}
