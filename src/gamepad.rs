use udev::*;
use std::ffi::{CString, CStr};
use std::fs::File;
use std::io::prelude::*;
use std::mem;
use libc as c;
use ioctl;

#[derive(Debug)]
pub struct Gilrs {
    gamepads: Vec<Gamepad>,
}

impl Gilrs {
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
        Gilrs { gamepads: gamepads }
    }

    pub fn pool_events(&mut self) -> EventIterator {
        self.gamepads[0].pool_events()
    }
}

#[derive(Debug)]
pub struct Gamepad {
    file: File,
    axes_info: AxesInfo,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct AxesInfo {
    abs_x_max: f32,
    abs_y_max: f32,
    abs_rx_max: f32,
    abs_ry_max: f32,
    abs_left_tr_max: f32,
    abs_right_tr_max: f32,
    abs_left_tr2_max: f32,
    abs_right_tr2_max: f32,
}


impl Gamepad {
    fn pool_events(&mut self) -> EventIterator {
        EventIterator(&mut self.file, &self.axes_info)
    }
}

pub struct EventIterator<'a>(&'a mut File, &'a AxesInfo);

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
            if event._type == EV_KEY {
                Button::from_u16(event.code).and_then(|btn| {
                    match event.value {
                        0 => Some(Event::ButtonReleased(btn)),
                        1 => Some(Event::ButtonPressed(btn)),
                        _ => None,
                    }
                })
            } else if event._type == EV_ABS {
                println!("Axis: {}", event.code);
                Axis::from_u16(event.code).map(|axis| {
                    let val = event.value as f32;
                    let val = match axis {
                        Axis::LeftStickX => val / self.1.abs_x_max,
                        Axis::LeftStickY => val / self.1.abs_y_max,
                        Axis::RightStickX => val / self.1.abs_rx_max,
                        Axis::RightStickY => val / self.1.abs_ry_max,
                        Axis::LeftTrigger => val / self.1.abs_left_tr_max,
                        Axis::LeftTrigger2 => val / self.1.abs_left_tr2_max,
                        Axis::RightTrigger => val / self.1.abs_right_tr_max,
                        Axis::RightTrigger2 => val / self.1.abs_right_tr2_max,
                    };
                    Event::AxisChanged(axis, val)
                })
            } else {
                None
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    ButtonPressed(Button),
    ButtonReleased(Button),
    AxisChanged(Axis, f32),
}

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq)]
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
    fn from_u16(btn: u16) -> Option<Self> {
        if btn >= BTN_SOUTH && btn <= BTN_THUMBR || btn >= BTN_DPAD_UP && btn <= BTN_DPAD_RIGHT {
            Some(unsafe { mem::transmute(btn) })
        } else {
            None
        }
    }
}

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Axis {
    LeftStickX = ABS_X,
    LeftStickY = ABS_Y,
    RightStickX = ABS_RX,
    RightStickY = ABS_RY,
    LeftTrigger = ABS_HAT1Y,
    LeftTrigger2 = ABS_HAT2Y,
    RightTrigger = ABS_HAT1X,
    RightTrigger2 = ABS_HAT2X,
}

impl Axis {
    fn from_u16(axis: u16) -> Option<Self> {
        if axis == ABS_X || axis == ABS_Y || axis == ABS_RX || axis == ABS_RY ||
           axis >= ABS_HAT1X && axis <= ABS_HAT2Y {
            Some(unsafe { mem::transmute(axis) })
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

        if ioctl::eviocgbit(fd, 0, EV_MAX as i32, ev_bits.as_mut_ptr()) < 0 ||
           ioctl::eviocgbit(fd, EV_KEY as u32, KEY_MAX as i32, key_bits.as_mut_ptr()) < 0 {
            c::close(fd);
            return None;
        }

        if !test_bit(BTN_GAMEPAD, &key_bits) {
            println!("{:?} doesn't have BTN_GAMEPAD, ignoring.", path);
            c::close(fd);
            return None;
        }

        let mut gamepad = Gamepad {
            file: File::open(path.to_str().unwrap()).unwrap(),
            axes_info: mem::zeroed(),
        };

        let mut absi = ioctl::input_absinfo::default();

        if ioctl::eviocgabs(fd, ABS_X as u32, &mut absi as *mut _) >= 0 {
            gamepad.axes_info.abs_x_max = absi.maximum as f32;
        }

        if ioctl::eviocgabs(fd, ABS_Y as u32, &mut absi as *mut _) >= 0 {
            gamepad.axes_info.abs_y_max = absi.maximum as f32;
        }

        if ioctl::eviocgabs(fd, ABS_RX as u32, &mut absi as *mut _) >= 0 {
            gamepad.axes_info.abs_rx_max = absi.maximum as f32;
        }

        if ioctl::eviocgabs(fd, ABS_RY as u32, &mut absi as *mut _) >= 0 {
            gamepad.axes_info.abs_ry_max = absi.maximum as f32;
        }

        if ioctl::eviocgabs(fd, ABS_HAT1X as u32, &mut absi as *mut _) >= 0 {
            gamepad.axes_info.abs_right_tr_max = absi.maximum as f32;
        }

        if ioctl::eviocgabs(fd, ABS_HAT1Y as u32, &mut absi as *mut _) >= 0 {
            gamepad.axes_info.abs_left_tr_max = absi.maximum as f32;
        }

        if ioctl::eviocgabs(fd, ABS_HAT2X as u32, &mut absi as *mut _) >= 0 {
            gamepad.axes_info.abs_right_tr2_max = absi.maximum as f32;
        }

        if ioctl::eviocgabs(fd, ABS_HAT2Y as u32, &mut absi as *mut _) >= 0 {
            gamepad.axes_info.abs_left_tr2_max = absi.maximum as f32;
        }

        println!("{:#?}", gamepad);
        // Use Rust IO for reading events
        c::close(fd);
        Some(gamepad)
    }
}

fn test_bit(n: u16, array: &[u8]) -> bool {
    (array[(n / 8) as usize] >> (n % 8)) & 1 != 0
}

const KEY_MAX: u16 = 0x2ff;
const EV_MAX: u16 = 0x1f;
const EV_KEY: u16 = 0x01;
const EV_ABS: u16 = 0x03;

const BTN_GAMEPAD: u16 = 0x130;
const BTN_SOUTH: u16 = 0x130;
const BTN_EAST: u16 = 0x131;
const BTN_C: u16 = 0x132;
const BTN_NORTH: u16 = 0x133;
const BTN_WEST: u16 = 0x134;
const BTN_Z: u16 = 0x135;
const BTN_TL: u16 = 0x136;
const BTN_TR: u16 = 0x137;
const BTN_TL2: u16 = 0x138;
const BTN_TR2: u16 = 0x139;
const BTN_SELECT: u16 = 0x13a;
const BTN_START: u16 = 0x13b;
const BTN_MODE: u16 = 0x13c;
const BTN_THUMBL: u16 = 0x13d;
const BTN_THUMBR: u16 = 0x13e;

const BTN_DPAD_UP: u16 = 0x220;
const BTN_DPAD_DOWN: u16 = 0x221;
const BTN_DPAD_LEFT: u16 = 0x222;
const BTN_DPAD_RIGHT: u16 = 0x223;

const ABS_X: u16 = 0x00;
const ABS_Y: u16 = 0x01;
const ABS_RX: u16 = 0x03;
const ABS_RY: u16 = 0x04;
const ABS_HAT1X: u16 = 0x12;
const ABS_HAT1Y: u16 = 0x13;
const ABS_HAT2X: u16 = 0x14;
const ABS_HAT2Y: u16 = 0x15;
