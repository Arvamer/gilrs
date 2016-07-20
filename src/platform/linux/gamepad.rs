use super::udev::*;
use std::ffi::{CString, CStr};
use std::mem;
use uuid::Uuid;
use libc as c;
use ioctl;
use gamepad::{Event, Button, Axis, Status};
use constants;
use mapping::{Mapping, Kind, MappingDb};


#[derive(Debug)]
pub struct Gilrs {
    pub gamepads: Vec<Gamepad>,
    mapping_db: MappingDb,
    monitor: Monitor,
}

impl Gilrs {
    pub fn new() -> Self {
        let mut gamepads = Vec::new();
        let mapping_db = MappingDb::new();

        let udev = Udev::new().unwrap();
        let en = udev.enumerate().unwrap();
        en.add_match_property(&CString::new("ID_INPUT_JOYSTICK").unwrap(),
                              &CString::new("1").unwrap());
        en.scan_devices();

        for dev in en.iter() {
            let dev = Device::from_syspath(&udev, &dev).unwrap();
            if let Some(gamepad) = Gamepad::open(&dev, &mapping_db) {
                gamepads.push(gamepad);
            }
        }
        Gilrs {
            gamepads: gamepads,
            mapping_db: mapping_db,
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
                if let Some(gamepad) = Gamepad::open(&dev, &self.mapping_db) {
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
    abs_dpad_prev_val: (i16, i16),
    mapping: Mapping,
    ff_supported: bool,
    devpath: String,
    pub name: String,
    pub uuid: Uuid,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct AxesInfo {
    abs_x_max: f32,
    abs_y_max: f32,
    abs_z_max: f32,
    abs_rx_max: f32,
    abs_ry_max: f32,
    abs_rz_max: f32,
    abs_dpadx_max: f32,
    abs_dpady_max: f32,
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
            abs_dpad_prev_val: (0, 0),
            mapping: Mapping::new(),
            ff_supported: false,
            devpath: String::new(),
            name: String::new(),
            uuid: Uuid::nil(),
        }
    }

    pub fn fd(&self) -> i32 {
        self.fd
    }

    fn dummy(dev: &Device) -> Option<Self> {
        dev.devnode().map(|devpath| {
            Gamepad {
                fd: -3,
                axes_info: unsafe { mem::uninitialized() },
                abs_dpad_prev_val: (0, 0),
                mapping: Mapping::new(),
                ff_supported: false,
                devpath: devpath.to_string_lossy().into_owned(),
                name: String::new(),
                uuid: Uuid::nil(),
            }
        })
    }

    fn open(dev: &Device, mapping_db: &MappingDb) -> Option<Gamepad> {
        let path = match dev.devnode() {
            Some(path) => path,
            None => return None,
        };

        unsafe {
            let fd = c::open(path.as_ptr(), c::O_RDWR | c::O_NONBLOCK);
            if fd < 0 {
                return None;
            }

            let mut ev_bits = [0u8; (EV_MAX / 8) as usize + 1];
            let mut key_bits = [0u8; (KEY_MAX / 8) as usize + 1];
            let mut abs_bits = [0u8; (ABS_MAX / 8) as usize + 1];

            if ioctl::eviocgbit(fd, 0, ev_bits.len() as i32, ev_bits.as_mut_ptr()) < 0 ||
               ioctl::eviocgbit(fd,
                                EV_KEY as u32,
                                key_bits.len() as i32,
                                key_bits.as_mut_ptr()) < 0 ||
               ioctl::eviocgbit(fd,
                                EV_ABS as u32,
                                abs_bits.len() as i32,
                                abs_bits.as_mut_ptr()) < 0 {
                c::close(fd);
                return None;
            }

            let mut buttons = Vec::with_capacity(16);
            let mut axes = Vec::with_capacity(8);

            for bit in 0..(key_bits.len() * 8) {
                if test_bit(bit as u16, &key_bits) {
                    buttons.push(bit as u16);
                }
            }
            for bit in 0..(abs_bits.len() * 8) {
                if test_bit(bit as u16, &abs_bits) {
                    axes.push(bit as u16);
                }
            }

            let mut namebuff = mem::uninitialized::<[u8; 128]>();
            let mut input_id = mem::uninitialized::<ioctl::input_id>();

            if ioctl::eviocgname(fd, namebuff.as_mut_ptr(), namebuff.len()) < 0 {
                return None;
            }

            if ioctl::eviocgid(fd, &mut input_id as *mut _) < 0 {
                return None;
            }


            let mut ff_bits = [0u8; (FF_MAX / 8) as usize + 1];
            let mut ff_supported = false;

            if ioctl::eviocgbit(fd, EV_FF as u32, ff_bits.len() as i32, ff_bits.as_mut_ptr()) >= 0 {
                if test_bit(FF_SQUARE, &ff_bits) && test_bit(FF_TRIANGLE, &ff_bits) &&
                   test_bit(FF_SINE, &ff_bits) && test_bit(FF_GAIN, &ff_bits) {
                    ff_supported = true;
                }
            }

            let mut absi = ioctl::input_absinfo::default();
            let mut axesi = mem::zeroed::<AxesInfo>();
            let uuid = create_uuid(input_id);
            let mapping = mapping_db.get(uuid)
                                    .and_then(|s| {
                                        Mapping::parse_sdl_mapping(s, &buttons, &axes).ok()
                                    })
                                    .unwrap_or(Mapping::new());

            println!("{:?}, {:?}", axes, mapping);
            if !test_bit(mapping.map_rev(BTN_GAMEPAD, Kind::Button), &key_bits) {
                println!("{:?} doesn't have BTN_GAMEPAD, ignoring.", path);
                c::close(fd);
                return None;
            }

            if ioctl::eviocgabs(fd,
                                mapping.map_rev(ABS_X, Kind::Axis) as u32,
                                &mut absi as *mut _) >= 0 {
                axesi.abs_x_max = absi.maximum as f32;
            }

            if ioctl::eviocgabs(fd,
                                mapping.map_rev(ABS_Y, Kind::Axis) as u32,
                                &mut absi as *mut _) >= 0 {
                axesi.abs_y_max = absi.maximum as f32;
            }

            if ioctl::eviocgabs(fd,
                                mapping.map_rev(ABS_Z, Kind::Axis) as u32,
                                &mut absi as *mut _) >= 0 {
                axesi.abs_z_max = absi.maximum as f32;
            }

            if ioctl::eviocgabs(fd,
                                mapping.map_rev(ABS_RX, Kind::Axis) as u32,
                                &mut absi as *mut _) >= 0 {
                axesi.abs_rx_max = absi.maximum as f32;
            }

            if ioctl::eviocgabs(fd,
                                mapping.map_rev(ABS_RY, Kind::Axis) as u32,
                                &mut absi as *mut _) >= 0 {
                axesi.abs_ry_max = absi.maximum as f32;
            }

            if ioctl::eviocgabs(fd,
                                mapping.map_rev(ABS_RZ, Kind::Axis) as u32,
                                &mut absi as *mut _) >= 0 {
                axesi.abs_rz_max = absi.maximum as f32;
            }

            if ioctl::eviocgabs(fd,
                                mapping.map_rev(ABS_HAT0X, Kind::Axis) as u32,
                                &mut absi as *mut _) >= 0 {
                axesi.abs_dpadx_max = absi.maximum as f32;
            }

            if ioctl::eviocgabs(fd,
                                mapping.map_rev(ABS_HAT0Y, Kind::Axis) as u32,
                                &mut absi as *mut _) >= 0 {
                axesi.abs_dpady_max = absi.maximum as f32;
            }

            if ioctl::eviocgabs(fd,
                                mapping.map_rev(ABS_HAT1X, Kind::Axis) as u32,
                                &mut absi as *mut _) >= 0 {
                axesi.abs_right_tr_max = absi.maximum as f32;
            }

            if ioctl::eviocgabs(fd,
                                mapping.map_rev(ABS_HAT1Y, Kind::Axis) as u32,
                                &mut absi as *mut _) >= 0 {
                axesi.abs_left_tr_max = absi.maximum as f32;
            }

            if ioctl::eviocgabs(fd,
                                mapping.map_rev(ABS_HAT2X, Kind::Axis) as u32,
                                &mut absi as *mut _) >= 0 {
                axesi.abs_right_tr2_max = absi.maximum as f32;
            }

            if ioctl::eviocgabs(fd,
                                mapping.map_rev(ABS_HAT2Y, Kind::Axis) as u32,
                                &mut absi as *mut _) >= 0 {
                axesi.abs_left_tr2_max = absi.maximum as f32;
            }

            let gamepad = Gamepad {
                fd: fd,
                axes_info: axesi,
                abs_dpad_prev_val: (0, 0),
                mapping: mapping,
                ff_supported: ff_supported,
                devpath: path.to_string_lossy().into_owned(),
                name: CStr::from_ptr(namebuff.as_ptr() as *const i8).to_string_lossy().into_owned(),
                uuid: uuid,
            };

            println!("{:#?}", gamepad);

            Some(gamepad)
        }
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


            let ev = match event._type {
                EV_KEY => {
                    let code = self.mapping.map(event.code, Kind::Button);
                    Button::from_u16(code).and_then(|btn| {
                        match event.value {
                            0 => Some(Event::ButtonReleased(btn)),
                            1 => Some(Event::ButtonPressed(btn)),
                            _ => None,
                        }
                    })
                }
                EV_ABS => {
                    let code = self.mapping.map(event.code, Kind::Axis);
                    match code {
                        ABS_HAT0Y => {
                            let ev = match event.value {
                                0 => {
                                    match self.abs_dpad_prev_val.1 {
                                        val if val > 0 => {
                                            Some(Event::ButtonReleased(Button::DPadDown))
                                        }
                                        val if val < 0 => {
                                            Some(Event::ButtonReleased(Button::DPadUp))
                                        }
                                        _ => None,
                                    }
                                }
                                val if val > 0 => Some(Event::ButtonPressed(Button::DPadDown)),
                                val if val < 0 => Some(Event::ButtonPressed(Button::DPadUp)),
                                _ => unreachable!(),
                            };
                            self.abs_dpad_prev_val.1 = event.value as i16;
                            ev
                        }
                        ABS_HAT0X => {
                            let ev = match event.value {
                                0 => {
                                    match self.abs_dpad_prev_val.0 {
                                        val if val > 0 => {
                                            Some(Event::ButtonReleased(Button::DPadRight))
                                        }
                                        val if val < 0 => {
                                            Some(Event::ButtonReleased(Button::DPadLeft))
                                        }
                                        _ => None,
                                    }
                                }
                                val if val > 0 => Some(Event::ButtonPressed(Button::DPadRight)),
                                val if val < 0 => Some(Event::ButtonPressed(Button::DPadLeft)),
                                _ => unreachable!(),
                            };
                            self.abs_dpad_prev_val.0 = event.value as i16;
                            ev
                        }
                        code => {
                            Axis::from_u16(code).map(|axis| {
                                let val = event.value as f32;
                                let val = match axis {
                                    Axis::LeftStickX => val / self.axes_info.abs_x_max,
                                    Axis::LeftStickY => val / self.axes_info.abs_y_max,
                                    Axis::LeftZ => val / self.axes_info.abs_z_max,
                                    Axis::RightStickX => val / self.axes_info.abs_rx_max,
                                    Axis::RightStickY => val / self.axes_info.abs_ry_max,
                                    Axis::RightZ => val / self.axes_info.abs_rz_max,
                                    Axis::LeftTrigger => val / self.axes_info.abs_left_tr_max,
                                    Axis::LeftTrigger2 => val / self.axes_info.abs_left_tr2_max,
                                    Axis::RightTrigger => val / self.axes_info.abs_right_tr_max,
                                    Axis::RightTrigger2 => val / self.axes_info.abs_right_tr2_max,
                                };
                                Event::AxisChanged(axis, val)
                            })
                        }
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

    pub fn max_ff_effects(&self) -> usize {
        if self.ff_supported {
            let mut max_effects = 0;
            unsafe {
                ioctl::eviocgeffects(self.fd, &mut max_effects as *mut _);
            }
            max_effects as usize
        } else {
            0
        }
    }

    pub fn is_ff_supported(&self) -> bool {
        self.ff_supported
    }

    pub fn set_ff_gain(&mut self, gain: u16) {
        let ev = ioctl::input_event {
            _type: EV_FF,
            code: FF_GAIN,
            value: gain as i32,
            time: unsafe { mem::uninitialized() },
        };
        unsafe {
            c::write(self.fd, mem::transmute(&ev), 24);
        }
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

impl PartialEq for Gamepad {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
    }
}

fn create_uuid(iid: ioctl::input_id) -> Uuid {
    let bus = (iid.bustype as u32).to_be();
    let vendor = iid.vendor.to_be();
    let product = iid.product.to_be();
    let version = iid.version.to_be();
    Uuid::from_fields(bus,
                      vendor,
                      0,
                      &[(product >> 8) as u8,
                        product as u8,
                        0,
                        0,
                        (version >> 8) as u8,
                        version as u8,
                        0,
                        0])
        .unwrap()
}

impl Button {
    fn from_u16(btn: u16) -> Option<Self> {
        if btn >= BTN_SOUTH && btn <= BTN_THUMBR {
            Some(unsafe { mem::transmute(btn - (BTN_SOUTH - constants::BTN_SOUTH)) })
        } else if btn >= BTN_DPAD_UP && btn <= BTN_DPAD_RIGHT {
            Some(unsafe { mem::transmute(btn - (BTN_DPAD_UP - constants::BTN_DPAD_UP)) })
        } else {
            None
        }
    }
}

impl Axis {
    fn from_u16(axis: u16) -> Option<Self> {
        if axis >= ABS_X && axis <= ABS_RZ {
            Some(unsafe { mem::transmute(axis) })
        } else if axis >= ABS_HAT1X && axis <= ABS_HAT2Y {
            Some(unsafe { mem::transmute(axis - 10) })
        } else {
            None
        }
    }
}

fn test_bit(n: u16, array: &[u8]) -> bool {
    (array[(n / 8) as usize] >> (n % 8)) & 1 != 0
}

const KEY_MAX: u16 = 0x2ff;
const EV_MAX: u16 = 0x1f;
const EV_KEY: u16 = 0x01;
const EV_ABS: u16 = 0x03;
const ABS_MAX: u16 = 0x3f;
const EV_FF: u16 = 0x15;

#[allow(dead_code)]
const BTN_MISC: u16 = 0x100;
#[allow(dead_code)]
const BTN_GAMEPAD: u16 = 0x130;
#[allow(dead_code)]
const BTN_SOUTH: u16 = 0x130;
#[allow(dead_code)]
const BTN_EAST: u16 = 0x131;
#[allow(dead_code)]
const BTN_C: u16 = 0x132;
#[allow(dead_code)]
const BTN_NORTH: u16 = 0x133;
#[allow(dead_code)]
const BTN_WEST: u16 = 0x134;
#[allow(dead_code)]
const BTN_Z: u16 = 0x135;
#[allow(dead_code)]
const BTN_TL: u16 = 0x136;
#[allow(dead_code)]
const BTN_TR: u16 = 0x137;
#[allow(dead_code)]
const BTN_TL2: u16 = 0x138;
#[allow(dead_code)]
const BTN_TR2: u16 = 0x139;
#[allow(dead_code)]
const BTN_SELECT: u16 = 0x13a;
#[allow(dead_code)]
const BTN_START: u16 = 0x13b;
#[allow(dead_code)]
const BTN_MODE: u16 = 0x13c;
#[allow(dead_code)]
const BTN_THUMBL: u16 = 0x13d;
#[allow(dead_code)]
const BTN_THUMBR: u16 = 0x13e;

#[allow(dead_code)]
const BTN_DPAD_UP: u16 = 0x220;
#[allow(dead_code)]
const BTN_DPAD_DOWN: u16 = 0x221;
#[allow(dead_code)]
const BTN_DPAD_LEFT: u16 = 0x222;
#[allow(dead_code)]
const BTN_DPAD_RIGHT: u16 = 0x223;

#[allow(dead_code)]
const ABS_X: u16 = 0x00;
#[allow(dead_code)]
const ABS_Y: u16 = 0x01;
#[allow(dead_code)]
const ABS_Z: u16 = 0x02;
#[allow(dead_code)]
const ABS_RX: u16 = 0x03;
#[allow(dead_code)]
const ABS_RY: u16 = 0x04;
#[allow(dead_code)]
const ABS_RZ: u16 = 0x05;
#[allow(dead_code)]
const ABS_HAT0X: u16 = 0x10;
#[allow(dead_code)]
const ABS_HAT0Y: u16 = 0x11;
#[allow(dead_code)]
const ABS_HAT1X: u16 = 0x12;
#[allow(dead_code)]
const ABS_HAT1Y: u16 = 0x13;
#[allow(dead_code)]
const ABS_HAT2X: u16 = 0x14;
#[allow(dead_code)]
const ABS_HAT2Y: u16 = 0x15;

#[allow(dead_code)]
const FF_MAX: u16 = FF_GAIN;
#[allow(dead_code)]
const FF_SQUARE: u16 = 0x58;
#[allow(dead_code)]
const FF_TRIANGLE: u16 = 0x59;
#[allow(dead_code)]
const FF_SINE: u16 = 0x5a;
#[allow(dead_code)]
#[allow(dead_code)]
const FF_GAIN: u16 = 0x60;

pub mod native_ev_codes {
    #![allow(dead_code)]
    pub const BTN_SOUTH: u16 = super::BTN_SOUTH;
    pub const BTN_EAST: u16 = super::BTN_EAST;
    pub const BTN_C: u16 = super::BTN_C;
    pub const BTN_NORTH: u16 = super::BTN_NORTH;
    pub const BTN_WEST: u16 = super::BTN_WEST;
    pub const BTN_Z: u16 = super::BTN_Z;
    pub const BTN_LT: u16 = super::BTN_TL;
    pub const BTN_RT: u16 = super::BTN_TR;
    pub const BTN_LT2: u16 = super::BTN_TL2;
    pub const BTN_RT2: u16 = super::BTN_TR2;
    pub const BTN_SELECT: u16 = super::BTN_SELECT;
    pub const BTN_START: u16 = super::BTN_START;
    pub const BTN_MODE: u16 = super::BTN_MODE;
    pub const BTN_LTHUMB: u16 = super::BTN_THUMBL;
    pub const BTN_RTHUMB: u16 = super::BTN_THUMBR;

    pub const BTN_DPAD_UP: u16 = super::BTN_DPAD_UP;
    pub const BTN_DPAD_DOWN: u16 = super::BTN_DPAD_DOWN;
    pub const BTN_DPAD_LEFT: u16 = super::BTN_DPAD_LEFT;
    pub const BTN_DPAD_RIGHT: u16 = super::BTN_DPAD_RIGHT;

    pub const AXIS_LSTICKX: u16 = super::ABS_X;
    pub const AXIS_LSTICKY: u16 = super::ABS_Y;
    pub const AXIS_LEFTZ: u16 = super::ABS_Z;
    pub const AXIS_RSTICKX: u16 = super::ABS_RX;
    pub const AXIS_RSTICKY: u16 = super::ABS_RY;
    pub const AXIS_RIGHTZ: u16 = super::ABS_RZ;
    pub const AXIS_DPADX: u16 = super::ABS_HAT0X;
    pub const AXIS_DPADY: u16 = super::ABS_HAT0Y;
    pub const AXIS_RT: u16 = super::ABS_HAT1X;
    pub const AXIS_LT: u16 = super::ABS_HAT1Y;
    pub const AXIS_RT2: u16 = super::ABS_HAT2X;
    pub const AXIS_LT2: u16 = super::ABS_HAT2Y;
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;
    use ioctl;

    #[test]
    fn sdl_uuid() {
        let x = Uuid::parse_str("030000005e0400008e02000020200000").unwrap();
        let y = super::create_uuid(ioctl::input_id {
            bustype: 0x3,
            vendor: 0x045e,
            product: 0x028e,
            version: 0x2020,
        });
        assert_eq!(x, y);
    }
}
