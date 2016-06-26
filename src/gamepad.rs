use udev::*;
use std::ffi::{CString, CStr};
use std::fs::File;
use std::io::prelude::*;
use std::mem;
use libc as c;
use ioctl;

pub struct Gamepads {
    gamepads: Vec<Gamepad>,
}

impl Gamepads {
    pub fn new() -> Self {
        let mut gamepads = Vec::new();

        let udev = Udev::new().unwrap();
        let en = udev.enumerate().unwrap();
        en.add_match_property(&CString::new("ID_INPUT_JOYSTICK").unwrap(),
                              &CString::new("1").unwrap());
        en.scan_devices();

        for dev in en.iter() {
            let dev = Device::from_syspath(&udev, &dev).unwrap();
            let devnode = dev.devnode();
            if devnode.is_none() {
                continue;
            }
            let devnode = devnode.unwrap();
            if let Some(gamepad) = open_and_check(devnode) {
                gamepads.push(gamepad);
            }
        }
        Gamepads { gamepads: gamepads }
    }

    pub fn pool_events(&mut self) -> EventIterator {
        self.gamepads[0].pool_events()
    }
}

pub struct Gamepad(File);

impl Gamepad {
    fn pool_events(&mut self) -> EventIterator {
        EventIterator(&mut self.0)
    }
}

pub struct EventIterator<'a>(&'a mut File);

impl<'a> Iterator for EventIterator<'a> {
    type Item = Event;

    fn next(&mut self) -> Option<Event> {
        let mut buff = [0; 24];
        let n = self.0.read(&mut buff).unwrap();
        if n == 0 {
            None
        } else if n != 24 {
            unimplemented!()
        } else {
            let event = unsafe { mem::transmute::<_, ioctl::input_event>(buff) };
            if event._type as u32 == EV_KEY {
                Button::from_u32(event.code as u32).and_then(|btn| {
                    match event.value {
                        0 => Some(Event::ButtonReleased(btn)),
                        1 => Some(Event::ButtonPressed(btn)),
                        _ => None,
                    }
                })
            } else {
                None
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    ButtonPressed(Button),
    ButtonReleased(Button),
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    // Action Pad
    South = BTN_SOUTH,
    East = BTN_EAST,
    North = BTN_NORTH,
    West = BTN_WEST,
    C = BTN_C,
    Z = BTN_Z,
    // Triggers
    LeftTrigger = BTN_TL,
    LeftTrigger2 = BTN_TL2,
    RightTrigger = BTN_TR,
    RightTrigger2 = BTN_TR2,
    // Menu Pad
    Select = BTN_SELECT,
    Start = BTN_START,
    Mode = BTN_MODE,
    // Sticks
    LeftThumb = BTN_THUMBL,
    RightThumb = BTN_THUMBR,
    // D-Pad
    DPadUP = BTN_DPAD_UP,
    DPadDown = BTN_DPAD_DOWN,
    DPadLeft = BTN_DPAD_LEFT,
    DPadRight = BTN_DPAD_RIGHT,
}

impl Button {
    fn from_u32(btn: u32) -> Option<Self> {
        if btn >= BTN_SOUTH && btn <= BTN_THUMBR ||
           btn >= BTN_DPAD_UP && btn <= BTN_DPAD_RIGHT {
            Some(unsafe { mem::transmute(btn) })
        } else {
            None
        }
    }
}

fn open_and_check(path: &CStr) -> Option<Gamepad> {
    unsafe {
        let fd = c::open(path.as_ptr(), c::O_RDONLY);
        if fd < 0 {
            return None;
        }

        let mut ev_bits = [0u8; EV_MAX as usize];
        let mut key_bits = [0u8; KEY_MAX as usize];
        let mut abs_bits = [0u8; 1];

        if ioctl::eviocgbit(fd, 0, EV_MAX as i32, ev_bits.as_mut_ptr()) < 0 ||
           ioctl::eviocgbit(fd, EV_KEY, KEY_MAX as i32, key_bits.as_mut_ptr()) < 0 ||
           ioctl::eviocgbit(fd, EV_ABS, 1, abs_bits.as_mut_ptr()) < 0 {
            c::close(fd);
            return None;
        }

        if !test_bit(EV_ABS, &ev_bits) || !test_bit(BTN_GAMEPAD, &key_bits) {
            c::close(fd);
            return None;
        }

        // Use Rust IO for reading events
        c::close(fd);
        Some(Gamepad(File::open(path.to_str().unwrap()).unwrap()))
    }
}

fn test_bit(n: u32, array: &[u8]) -> bool {
    (array[(n / 8) as usize] >> (n % 8)) & 1 != 0
}

const KEY_MAX: u32 = 0x2ff;
const EV_MAX: u32 = 0x1f;
const EV_KEY: u32 = 0x01;
const EV_ABS: u32 = 0x03;

const BTN_GAMEPAD: u32 = 0x130;
const BTN_SOUTH: u32 = 0x130;
const BTN_EAST: u32 = 0x131;
const BTN_C: u32 = 0x132;
const BTN_NORTH: u32 = 0x133;
const BTN_WEST: u32 = 0x134;
const BTN_Z: u32 = 0x135;
const BTN_TL: u32 = 0x136;
const BTN_TR: u32 = 0x137;
const BTN_TL2: u32 = 0x138;
const BTN_TR2: u32 = 0x139;
const BTN_SELECT: u32 = 0x13a;
const BTN_START: u32 = 0x13b;
const BTN_MODE: u32 = 0x13c;
const BTN_THUMBL: u32 = 0x13d;
const BTN_THUMBR: u32 = 0x13e;

const BTN_DPAD_UP: u32 = 0x220;
const BTN_DPAD_DOWN: u32 = 0x221;
const BTN_DPAD_LEFT: u32 = 0x222;
const BTN_DPAD_RIGHT: u32 = 0x223;
