// TODO: Do something with linux constants.
#![allow(dead_code)]

use super::udev::*;
use std::ffi::{CString, CStr};
use std::mem;
use vec_map::VecMap;
use libc as c;
use ioctl;
use gamepad::{Event, Button, Axis, Status};


#[derive(Debug)]
pub struct Gilrs {
    pub gamepads: Vec<Gamepad>,
    monitor: Monitor,
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
            if let Some(gamepad) = open_and_check(&dev) {
                gamepads.push(gamepad);
            }
        }
        Gilrs {
            gamepads: gamepads,
            monitor: Monitor::new(&udev).unwrap(),
        }
    }

    pub fn handle_hotplug(&mut self) -> Option<(Gamepad, Status)> {
        while self.monitor.hotplug_available() {
            let dev = self.monitor.device();

            if let Some(val) = dev.property_value(&CString::new("ID_INPUT_JOYSTICK").unwrap()) {
                if !is_eq_cstr(val, b"1\0") {
                    continue;
                }
            } else {
                continue;
            }

            let action = dev.action().unwrap();

            if is_eq_cstr(action, b"add\0") {
                if let Some(gamepad) = open_and_check(&dev) {
                    return Some((gamepad, Status::Connected));
                }
            } else if is_eq_cstr(action, b"remove\0") {
                if let Some(gamepad) = Gamepad::dummy(&dev) {
                    return Some((gamepad, Status::Disconnected));
                }
            }
        }
        None
    }
}

fn is_eq_cstr(l: &CStr, r: &[u8]) -> bool {
    unsafe { c::strcmp(l.as_ptr(), r.as_ptr() as *const i8) == 0 }
}

#[derive(Debug)]
pub struct Gamepad {
    fd: i32,
    axes_info: AxesInfo,
    mapping: Mapping,
    id: (u16, u16),
    devpath: String,
    pub name: String,
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
    pub fn none() -> Self {
        Gamepad {
            fd: -3,
            axes_info: unsafe { mem::zeroed() },
            mapping: Mapping::new(),
            id: (0, 0),
            devpath: String::new(),
            name: String::new(),
        }
    }

    fn dummy(dev: &Device) -> Option<Self> {
        dev.devnode().map(|devpath| {
            Gamepad {
                fd: -3,
                axes_info: unsafe { mem::uninitialized() },
                mapping: Mapping::new(),
                id: (0, 0),
                devpath: devpath.to_string_lossy().into_owned(),
                name: String::new(),
            }
        })
    }

    pub fn eq_disconnect(&self, other: &Self) -> bool {
        self.devpath == other.devpath
    }

    pub fn event(&mut self) -> Option<Event> {
        let mut event = unsafe { mem::uninitialized::<ioctl::input_event>() };
        // Skip all unknown events and return Option on first know event or when there is no more
        // events to read. Returning None on unknown event breaks iterators.
        loop {
            let n = unsafe { c::read(self.fd, mem::transmute(&mut event), 24) };

            if n == -1 || n == 0 {
                // Nothing to read (non-blocking IO)
                return None;
            } else if n != 24 {
                unreachable!()
            }

            let code = self.mapping.map(event.code, event._type);

            let ev = match event._type {
                EV_KEY => {
                    Button::from_u16(code).and_then(|btn| {
                        match event.value {
                            0 => Some(Event::ButtonReleased(btn)),
                            1 => Some(Event::ButtonPressed(btn)),
                            _ => None,
                        }
                    })
                }
                EV_ABS => {
                    if code == ABS_HAT0X || code == ABS_HAT0Y {
                        match event.value {
                            -1 if code == ABS_HAT0X => Some(Event::ButtonPressed(Button::DPadLeft)),
                            -1 if code == ABS_HAT0Y => Some(Event::ButtonPressed(Button::DPadUp)),
                            1 if code == ABS_HAT0X => Some(Event::ButtonPressed(Button::DPadRight)),
                            1 if code == ABS_HAT0Y => Some(Event::ButtonPressed(Button::DPadDown)),
                            // FIXME: Generate release event for each pressed button
                            0 if code == ABS_HAT0X => {
                                Some(Event::ButtonReleased(Button::DPadRight))
                            }
                            0 if code == ABS_HAT0Y => Some(Event::ButtonReleased(Button::DPadUp)),
                            _ => None,
                        }
                    } else {
                        Axis::from_u16(code).map(|axis| {
                            let val = event.value as f32;
                            let val = match axis {
                                Axis::LeftStickX => val / self.axes_info.abs_x_max,
                                Axis::LeftStickY => val / self.axes_info.abs_y_max,
                                Axis::RightStickX => val / self.axes_info.abs_rx_max,
                                Axis::RightStickY => val / self.axes_info.abs_ry_max,
                                Axis::LeftTrigger => val / self.axes_info.abs_left_tr_max,
                                Axis::LeftTrigger2 => val / self.axes_info.abs_left_tr2_max,
                                Axis::RightTrigger => val / self.axes_info.abs_right_tr_max,
                                Axis::RightTrigger2 => val / self.axes_info.abs_right_tr2_max,
                            };
                            Event::AxisChanged(axis, val)
                        })
                    }
                }
                _ => None,
            };
            if ev.is_none() {
                continue;
            }
            return ev;
        }
    }

    pub fn disconnect(&mut self) {
        unsafe {
            if self.fd >= 0 {
                c::close(self.fd);
            }
        }
        self.fd = -2;
        self.devpath.clear();
    }
}

impl Drop for Gamepad {
    fn drop(&mut self) {
        unsafe {
            if self.fd >= 0 {
                c::close(self.fd);
            }
        }
    }
}

#[derive(Debug)]
struct Mapping {
    axes: VecMap<u16>,
    // to save some memory, key is button code - BTN_MISC
    btns: VecMap<u16>,
}

impl Mapping {
    fn new() -> Self {
        Mapping {
            axes: VecMap::new(),
            btns: VecMap::new(),
        }
    }
    fn map(&self, code: u16, kind: u16) -> u16 {
        match kind {
            EV_KEY => *self.btns.get((code - BTN_MISC) as usize).unwrap_or(&code),
            EV_ABS => *self.axes.get(code as usize).unwrap_or(&code),
            _ => code,
        }
    }

    fn map_rev(&self, code: u16, kind: u16) -> u16 {
        match kind {
            EV_KEY => {
                self.btns
                    .iter()
                    .find(|x| *x.1 == code - BTN_MISC)
                    .unwrap_or((code as usize, &0))
                    .0 as u16 + BTN_MISC
            }
            EV_ABS => {
                self.axes.iter().find(|x| *x.1 == code).unwrap_or((code as usize, &0)).0 as u16
            }
            _ => code,
        }
    }
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

fn open_and_check(dev: &Device) -> Option<Gamepad> {
    let path = match dev.devnode() {
        Some(path) => path,
        None => return None,
    };

    unsafe {
        let fd = c::open(path.as_ptr(), c::O_RDONLY | c::O_NONBLOCK);
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

        let mut id_model = 0u16;
        let mut id_vendor = 0u16;
        let mut name = String::new();

        for (key, val) in dev.properties() {
            if key == "ID_MODEL_ID" {
                id_model = u16::from_str_radix(&val, 16).unwrap_or(0);
            }
            if key == "ID_VENDOR_ID" {
                id_vendor = u16::from_str_radix(&val, 16).unwrap_or(0);
            }
            if key == "ID_MODEL" {
                name = val;
            }
        }

        let mapping = get_mapping(id_vendor, id_model);

        if !test_bit(BTN_GAMEPAD, &key_bits) {
            println!("{:?} doesn't have BTN_GAMEPAD, ignoring.", path);
            c::close(fd);
            return None;
        }

        let mut absi = ioctl::input_absinfo::default();
        let mut axesi = mem::zeroed::<AxesInfo>();

        if ioctl::eviocgabs(fd,
                            mapping.map_rev(ABS_X, EV_ABS) as u32,
                            &mut absi as *mut _) >= 0 {
            axesi.abs_x_max = absi.maximum as f32;
        }

        if ioctl::eviocgabs(fd,
                            mapping.map_rev(ABS_Y, EV_ABS) as u32,
                            &mut absi as *mut _) >= 0 {
            axesi.abs_y_max = absi.maximum as f32;
        }

        if ioctl::eviocgabs(fd,
                            mapping.map_rev(ABS_RX, EV_ABS) as u32,
                            &mut absi as *mut _) >= 0 {
            axesi.abs_rx_max = absi.maximum as f32;
        }

        if ioctl::eviocgabs(fd,
                            mapping.map_rev(ABS_RY, EV_ABS) as u32,
                            &mut absi as *mut _) >= 0 {
            axesi.abs_ry_max = absi.maximum as f32;
        }

        if ioctl::eviocgabs(fd,
                            mapping.map_rev(ABS_HAT1X, EV_ABS) as u32,
                            &mut absi as *mut _) >= 0 {
            axesi.abs_right_tr_max = absi.maximum as f32;
        }

        if ioctl::eviocgabs(fd,
                            mapping.map_rev(ABS_HAT1Y, EV_ABS) as u32,
                            &mut absi as *mut _) >= 0 {
            axesi.abs_left_tr_max = absi.maximum as f32;
        }

        if ioctl::eviocgabs(fd,
                            mapping.map_rev(ABS_HAT2X, EV_ABS) as u32,
                            &mut absi as *mut _) >= 0 {
            axesi.abs_right_tr2_max = absi.maximum as f32;
        }

        if ioctl::eviocgabs(fd,
                            mapping.map_rev(ABS_HAT2Y, EV_ABS) as u32,
                            &mut absi as *mut _) >= 0 {
            axesi.abs_left_tr2_max = absi.maximum as f32;
        }

        let gamepad = Gamepad {
            fd: fd,
            axes_info: axesi,
            mapping: mapping,
            id: (id_vendor, id_model),
            devpath: path.to_string_lossy().into_owned(),
            name: name,
        };

        println!("{:#?}", gamepad);

        Some(gamepad)
    }
}

fn get_mapping(vendor: u16, model: u16) -> Mapping {
    let mut mapping = Mapping::new();

    match vendor {
        0x045e => {
            match model {
                0x028e => {
                    mapping.btns.insert((BTN_WEST - BTN_MISC) as usize, BTN_NORTH);
                    mapping.btns.insert((BTN_NORTH - BTN_MISC) as usize, BTN_WEST);
                    mapping.axes.insert(5, ABS_HAT2X);
                    mapping.axes.insert(2, ABS_HAT2Y);
                }
                _ => (),
            }
        }
        _ => (),
    };

    mapping
}

fn test_bit(n: u16, array: &[u8]) -> bool {
    (array[(n / 8) as usize] >> (n % 8)) & 1 != 0
}


const KEY_MAX: u16 = 0x2ff;
const EV_MAX: u16 = 0x1f;
const EV_KEY: u16 = 0x01;
const EV_ABS: u16 = 0x03;

const BTN_MISC: u16 = 0x100;
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
const ABS_HAT0X: u16 = 0x10;
const ABS_HAT0Y: u16 = 0x11;
const ABS_HAT1X: u16 = 0x12;
const ABS_HAT1Y: u16 = 0x13;
const ABS_HAT2X: u16 = 0x14;
const ABS_HAT2Y: u16 = 0x15;
