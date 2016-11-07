// Copyright 2016 GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use super::udev::*;
use AsInner;
use gamepad::{Event, Button, Axis, Status, Gamepad as MainGamepad, PowerInfo, GamepadImplExt,
    Deadzones};
use std::ffi::CStr;
use std::mem;
use std::str;
use uuid::Uuid;
use libc as c;
use ioctl;
use constants;
use mapping::{Mapping, Kind, MappingDb};
use ioctl::input_absinfo as AbsInfo;
use super::ioctl_def;


#[derive(Debug)]
pub struct Gilrs {
    gamepads: Vec<MainGamepad>,
    mapping_db: MappingDb,
    monitor: Option<Monitor>,
    not_observed: MainGamepad,
    event_counter: usize,
}

impl Gilrs {
    pub fn new() -> Self {
        let mut gamepads = Vec::new();
        let mapping_db = MappingDb::new();

        let udev = match Udev::new() {
            Some(udev) => udev,
            None => {
                error!("Failed to create udev context");
                return Self::none();
            }
        };
        let en = match udev.enumerate() {
            Some(en) => en,
            None => {
                error!("Failed to create udev enumerate object");
                return Self::none();
            }
        };

        unsafe { en.add_match_property(cstr_new(b"ID_INPUT_JOYSTICK\0"), cstr_new(b"1\0")) }
        en.scan_devices();

        for dev in en.iter() {
            if let Some(dev) = Device::from_syspath(&udev, &dev) {
                if let Some(gamepad) = Gamepad::open(&dev, &mapping_db) {
                    let ainfo = gamepad.axes_info;
                    gamepads.push(MainGamepad::from_inner_status(gamepad,
                                                                 Status::Connected,
                                                                 ainfo.into()));
                }
            }
        }

        let monitor = Monitor::new(&udev);
        if monitor.is_none() {
            error!("Failed to create udev monitor. Hotplugging will not be supported");
        }

        Gilrs {
            gamepads: gamepads,
            mapping_db: mapping_db,
            monitor: monitor,
            not_observed: MainGamepad::from_inner_status(Gamepad::none(),
                                                         Status::NotObserved,
                                                         Default::default()),
            event_counter: 0,
        }
    }

    fn none() -> Self {
        Gilrs {
            gamepads: Vec::new(),
            mapping_db: MappingDb::new(),
            monitor: None,
            not_observed: MainGamepad::from_inner_status(Gamepad::none(),
                                                         Status::NotObserved,
                                                         Default::default()),
            event_counter: 0,
        }
    }

    pub fn next_event(&mut self) -> Option<(usize, Event)> {
        // If there is hotplug event return it, otherwise loop over all gamepdas checking if there
        // is some event.
        if let Some((id, ev)) = self.handle_hotplug() {
            return Some((id, ev));
        }

        loop {
            let mut gamepad = match self.gamepads.get_mut(self.event_counter) {
                Some(gp) => gp,
                None => {
                    self.event_counter = 0;
                    return None;
                }
            };

            if gamepad.status() != Status::Connected {
                self.event_counter += 1;
                continue;
            }

            match gamepad.as_inner_mut().event() {
                Some(ev) => return Some((self.event_counter, ev)),
                None => {
                    self.event_counter += 1;
                    continue;
                }
            };
        }
    }

    pub fn gamepad(&self, id: usize) -> &MainGamepad {
        self.gamepads.get(id).unwrap_or(&self.not_observed)
    }

    pub fn gamepad_mut(&mut self, id: usize) -> &mut MainGamepad {
        self.gamepads.get_mut(id).unwrap_or(&mut self.not_observed)
    }

    pub fn last_gamepad_hint(&self) -> usize {
        self.gamepads.len()
    }

    fn handle_hotplug(&mut self) -> Option<(usize, Event)> {
        let monitor = match self.monitor {
            Some(ref m) => m,
            None => return None,
        };

        while monitor.hotplug_available() {
            let dev = monitor.device();

            unsafe {
                if let Some(val) = dev.property_value(cstr_new(b"ID_INPUT_JOYSTICK\0")) {
                    if val != cstr_new(b"1\0") {
                        continue;
                    }
                } else {
                    continue;
                }

                let action = match dev.action() {
                    Some(a) => a,
                    None => continue,
                };

                if action == cstr_new(b"add\0") {
                    if let Some(gamepad) = Gamepad::open(&dev, &self.mapping_db) {
                        if let Some(id) = self.gamepads.iter().position(|gp| {
                            gp.uuid() == gamepad.uuid && gp.status() == Status::Disconnected
                        }) {
                            let ainfo = gamepad.axes_info;
                            self.gamepads[id] = MainGamepad::from_inner_status(gamepad,
                                                                               Status::Connected,
                                                                               ainfo.into());
                            return Some((id, Event::Connected));
                        } else {
                            let ainfo = gamepad.axes_info;
                            self.gamepads
                                .push(MainGamepad::from_inner_status(gamepad,
                                                                     Status::Connected,
                                                                     ainfo.into()));
                            return Some((self.gamepads.len() - 1, Event::Connected));
                        }
                    }
                } else if action == cstr_new(b"remove\0") {
                    if let Some(devnode) = dev.devnode() {
                        if let Some(id) = self.gamepads
                            .iter()
                            .position(|gp| {
                                is_eq_cstr_str(devnode, &gp.as_inner().devpath) && gp.is_connected()
                            }) {
                            self.gamepads[id].as_inner_mut().disconnect();
                            return Some((id, Event::Disconnected));
                        } else {
                            info!("Could not find disconnect gamepad {:?}", devnode);
                        }
                    }
                }
            }
        }
        None
    }
}

fn is_eq_cstr_str(l: &CStr, r: &str) -> bool {
    unsafe {
        let mut l_ptr = l.as_ptr();
        let mut r_ptr = r.as_ptr();
        let end = r_ptr.offset(r.len() as isize);
        while *l_ptr != 0 && r_ptr != end {
            if *l_ptr != *r_ptr as i8 {
                return false;
            }
            l_ptr = l_ptr.offset(1);
            r_ptr = r_ptr.offset(1);
        }
        if *l_ptr == 0 && r_ptr == end { true } else { false }
    }
}

#[derive(Debug)]
pub struct Gamepad {
    fd: i32,
    axes_info: AxesInfo,
    abs_dpad_prev_val: (i16, i16),
    mapping: Mapping,
    ff_supported: bool,
    devpath: String,
    name: String,
    uuid: Uuid,
    bt_capacity_fd: i32,
    bt_status_fd: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct AxesInfo {
    x: AbsInfo,
    y: AbsInfo,
    z: AbsInfo,
    rx: AbsInfo,
    ry: AbsInfo,
    rz: AbsInfo,
    dpadx: AbsInfo,
    dpady: AbsInfo,
    left_tr: AbsInfo,
    right_tr: AbsInfo,
    left_tr2: AbsInfo,
    right_tr2: AbsInfo,
}

impl AxesInfo {
    fn normalize(&mut self) {
        // Some devices report sticks value in range [0, Max], and some in range [-Max, Max]
        Self::normalize_abs(&mut self.x);
        Self::normalize_abs(&mut self.y);
        Self::normalize_abs(&mut self.rx);
        Self::normalize_abs(&mut self.ry);
    }

    fn normalize_abs(abs: &mut AbsInfo) {
        if abs.minimum == 0 {
            abs.maximum /= 2;
            // Don't change minimum value, it allow to see if reported axis value should also be
            // modified
        }
    }
}

impl From<AxesInfo> for Deadzones {
    fn from(f: AxesInfo) -> Self {
        Deadzones {
            right_stick: f.x.flat as f32 / f.x.maximum as f32,
            left_stick: f.rx.flat as f32 / f.rx.maximum as f32,
            left_z: f.z.flat as f32 / f.z.maximum as f32,
            right_z: f.rz.flat as f32 / f.rx.maximum as f32,
            right_trigger: f.right_tr.flat as f32 / f.right_tr.maximum as f32,
            right_trigger2: f.right_tr2.flat as f32 / f.right_tr2.maximum as f32,
            left_trigger: f.left_tr.flat as f32 / f.left_tr.maximum as f32,
            left_trigger2: f.left_tr2.flat as f32 / f.left_tr2.maximum as f32,
        }
    }
}

impl Gamepad {
    fn none() -> Self {
        Gamepad {
            fd: -3,
            axes_info: unsafe { mem::zeroed() },
            abs_dpad_prev_val: (0, 0),
            mapping: Mapping::new(),
            ff_supported: false,
            devpath: String::new(),
            name: String::new(),
            uuid: Uuid::nil(),
            bt_status_fd: -1,
            bt_capacity_fd: -1,
        }
    }

    pub fn fd(&self) -> i32 {
        self.fd
    }

    fn open(dev: &Device, mapping_db: &MappingDb) -> Option<Gamepad> {
        let path = match dev.devnode() {
            Some(path) => path,
            None => return None,
        };

        unsafe {
            let fd = c::open(path.as_ptr(), c::O_RDWR | c::O_NONBLOCK);
            if fd < 0 {
                error!("Failed to open {:?}", path);
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
                info!("Unable to get essential information about device {:?}, probably js \
                       interface, skipping…",
                      path);
                return None;
            }

            let mut buttons = Vec::with_capacity(16);
            let mut axes = Vec::with_capacity(8);

            for bit in (BTN_MISC)..(BTN_MOUSE) {
                if test_bit(bit, &key_bits) {
                    buttons.push(bit);
                }
            }
            for bit in (BTN_JOYSTICK)..(key_bits.len() as u16 * 8) {
                if test_bit(bit, &key_bits) {
                    buttons.push(bit);
                }
            }
            for bit in 0..(abs_bits.len() * 8) {
                if test_bit(bit as u16, &abs_bits) {
                    axes.push(bit as u16);
                }
            }

            debug!("{:?}", buttons);

            let mut namebuff = mem::uninitialized::<[u8; 128]>();
            let mut input_id = mem::uninitialized::<ioctl::input_id>();

            if ioctl::eviocgname(fd, namebuff.as_mut_ptr(), namebuff.len()) < 0 {
                error!("Failed to get name of device {:?}", path);
                return None;
            }

            if ioctl_def::eviocgid(fd, &mut input_id as *mut _) < 0 {
                error!("Failed to get id of device {:?}", path);
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

            let mut axesi = mem::zeroed::<AxesInfo>();
            let uuid = create_uuid(input_id);
            let mapping = mapping_db.get(uuid)
                .and_then(|s| Mapping::parse_sdl_mapping(s, &buttons, &axes).ok())
                .unwrap_or(Mapping::new());

            let name = if mapping.name().is_empty() {
                CStr::from_ptr(namebuff.as_ptr() as *const i8).to_string_lossy().into_owned()
            } else {
                mapping.name().to_owned()
            };

            if !test_bit(mapping.map_rev(BTN_GAMEPAD, Kind::Button), &key_bits) {
                warn!("{:?}({}) doesn't have BTN_GAMEPAD, ignoring.", path, name);
                c::close(fd);
                return None;
            }

            ioctl::eviocgabs(fd,
                             mapping.map_rev(ABS_X, Kind::Axis) as u32,
                             &mut axesi.x as *mut _);

            ioctl::eviocgabs(fd,
                             mapping.map_rev(ABS_Y, Kind::Axis) as u32,
                             &mut axesi.y as *mut _);

            ioctl::eviocgabs(fd,
                             mapping.map_rev(ABS_Z, Kind::Axis) as u32,
                             &mut axesi.z as *mut _);

            ioctl::eviocgabs(fd,
                             mapping.map_rev(ABS_RX, Kind::Axis) as u32,
                             &mut axesi.rx as *mut _);

            ioctl::eviocgabs(fd,
                             mapping.map_rev(ABS_RY, Kind::Axis) as u32,
                             &mut axesi.ry as *mut _);

            ioctl::eviocgabs(fd,
                             mapping.map_rev(ABS_RZ, Kind::Axis) as u32,
                             &mut axesi.rz as *mut _);

            ioctl::eviocgabs(fd,
                             mapping.map_rev(ABS_HAT0X, Kind::Axis) as u32,
                             &mut axesi.dpadx as *mut _);

            ioctl::eviocgabs(fd,
                             mapping.map_rev(ABS_HAT0Y, Kind::Axis) as u32,
                             &mut axesi.dpady as *mut _);

            ioctl::eviocgabs(fd,
                             mapping.map_rev(ABS_HAT1X, Kind::Axis) as u32,
                             &mut axesi.right_tr as *mut _);

            ioctl::eviocgabs(fd,
                             mapping.map_rev(ABS_HAT1Y, Kind::Axis) as u32,
                             &mut axesi.left_tr as *mut _);

            ioctl::eviocgabs(fd,
                             mapping.map_rev(ABS_HAT2X, Kind::Axis) as u32,
                             &mut axesi.right_tr2 as *mut _);

            ioctl::eviocgabs(fd,
                             mapping.map_rev(ABS_HAT2Y, Kind::Axis) as u32,
                             &mut axesi.left_tr2 as *mut _);

            axesi.normalize();
            let (cap, status) = Self::battery_fd(&dev);

            let gamepad = Gamepad {
                fd: fd,
                axes_info: axesi,
                abs_dpad_prev_val: (0, 0),
                mapping: mapping,
                ff_supported: ff_supported,
                devpath: path.to_string_lossy().into_owned(),
                name: name,
                uuid: uuid,
                bt_capacity_fd: cap,
                bt_status_fd: status,
            };

            info!("Found {:#?}", gamepad);

            Some(gamepad)
        }
    }

    fn battery_fd(dev: &Device) -> (i32, i32) {
        use std::fs::{self, File};
        use std::path::Path;
        use std::os::unix::io::IntoRawFd;
        use std::os::unix::ffi::OsStrExt;
        use std::ffi::OsStr;

        let syspath = Path::new(OsStr::from_bytes(dev.syspath().to_bytes()));
        // Returned syspath points to <device path>/input/inputXX/eventXX. First "device" is
        // symlink to inputXX, second to actual device root.
        let syspath = syspath.join("device/device/power_supply");
        if let Ok(mut read_dir) = fs::read_dir(syspath) {
            if let Some(Ok(bat_entry)) = read_dir.next() {
                if let Ok(cap) = File::open(bat_entry.path().join("capacity")) {
                    if let Ok(status) = File::open(bat_entry.path().join("status")) {
                        return (cap.into_raw_fd(), status.into_raw_fd());
                    }
                }
            }
        }
        (-1, -1)
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
                    let btn = Button::from_u16(code);
                    match event.value {
                        0 => Some(Event::ButtonReleased(btn, event.code)),
                        1 => Some(Event::ButtonPressed(btn, event.code)),
                        _ => None,
                    }
                }
                EV_ABS => {
                    let code = self.mapping.map(event.code, Kind::Axis);
                    match code {
                        ABS_HAT0Y => {
                            let ev = match event.value {
                                0 => {
                                    match self.abs_dpad_prev_val.1 {
                                        val if val > 0 => {
                                            Some(Event::ButtonReleased(Button::DPadDown, event.code))
                                        }
                                        val if val < 0 => {
                                            Some(Event::ButtonReleased(Button::DPadUp, event.code))
                                        }
                                        _ => None,
                                    }
                                }
                                val if val > 0 => Some(Event::ButtonPressed(Button::DPadDown, event.code)),
                                val if val < 0 => Some(Event::ButtonPressed(Button::DPadUp, event.code)),
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
                                            Some(Event::ButtonReleased(Button::DPadRight, event.code))
                                        }
                                        val if val < 0 => {
                                            Some(Event::ButtonReleased(Button::DPadLeft, event.code))
                                        }
                                        _ => None,
                                    }
                                }
                                val if val > 0 => Some(Event::ButtonPressed(Button::DPadRight, event.code)),
                                val if val < 0 => Some(Event::ButtonPressed(Button::DPadLeft, event.code)),
                                _ => unreachable!(),
                            };
                            self.abs_dpad_prev_val.0 = event.value as i16;
                            ev
                        }
                        code => {
                            let axis = Axis::from_u16(code);
                            let ai = &self.axes_info;
                            let val = event.value;
                            let val = match axis {
                                a @ Axis::LeftStickX => Self::axis_value(ai.x, val, a),
                                a @ Axis::LeftStickY => Self::axis_value(ai.y, val, a),
                                a @ Axis::LeftZ => Self::axis_value(ai.z, val, a),
                                a @ Axis::RightStickX => Self::axis_value(ai.rx, val, a),
                                a @ Axis::RightStickY => Self::axis_value(ai.ry, val, a),
                                a @ Axis::RightZ => Self::axis_value(ai.rz, val, a),
                                a @ Axis::LeftTrigger => Self::axis_value(ai.left_tr, val, a),
                                a @ Axis::LeftTrigger2 => Self::axis_value(ai.left_tr2, val, a),
                                a @ Axis::RightTrigger => Self::axis_value(ai.right_tr, val, a),
                                a @ Axis::RightTrigger2 => Self::axis_value(ai.right_tr2, val, a),
                                Axis::Unknown => val as f32,
                            };
                            Some(Event::AxisChanged(axis, val, event.code))
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

    fn axis_value(axes_info: AbsInfo, val: i32, kind: Axis) -> f32 {
        let val =
            if kind.is_stick() && axes_info.minimum == 0 { val - axes_info.maximum } else { val };

        let val = val as f32 / axes_info.maximum as f32;

        val * if kind == Axis::LeftStickY || kind == Axis::RightStickY { -1.0 } else { 1.0 }
    }

    fn disconnect(&mut self) {
        unsafe {
            if self.fd >= 0 {
                c::close(self.fd);
            }
        }
        self.fd = -2;
        self.devpath.clear();
    }

    pub fn power_info(&self) -> PowerInfo {
        if self.bt_capacity_fd > -1 && self.bt_status_fd > -1 {
            unsafe {
                let mut buff = [0u8; 15];
                c::lseek(self.bt_capacity_fd, 0, c::SEEK_SET);
                c::lseek(self.bt_status_fd, 0, c::SEEK_SET);

                let len = c::read(self.bt_capacity_fd,
                                  mem::transmute(buff.as_mut_ptr()),
                                  buff.len()) as usize;

                if len > 0 {
                    let cap = match str::from_utf8_unchecked(&buff[..(len - 1)]).parse() {
                        Ok(cap) => cap,
                        Err(_) => {
                            error!("Failed to parse battery capacity: {}",
                                   str::from_utf8_unchecked(&buff[..(len - 1)]));
                            return PowerInfo::Unknown;
                        }
                    };

                    let len = c::read(self.bt_status_fd,
                                      mem::transmute(buff.as_mut_ptr()),
                                      buff.len()) as usize;

                    if len > 0 {
                        return match str::from_utf8_unchecked(&buff[..(len - 1)]) {
                            "Charging" => PowerInfo::Charging(cap),
                            "Discharging" => PowerInfo::Discharging(cap),
                            "Full" | "Not charging" => PowerInfo::Charged,
                            s => {
                                error!("Unknown battery status value: {}", s);
                                PowerInfo::Unknown
                            }
                        };
                    }
                }
            }
            PowerInfo::Unknown
        } else {
            if self.fd > -1 { PowerInfo::Wired } else { PowerInfo::Unknown }
        }
    }

    pub fn max_ff_effects(&self) -> usize {
        if self.ff_supported {
            let mut max_effects = 0;
            unsafe {
                ioctl_def::eviocgeffects(self.fd, &mut max_effects as *mut _);
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

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn uuid(&self) -> Uuid {
        self.uuid
    }
}

impl Drop for Gamepad {
    fn drop(&mut self) {
        unsafe {
            if self.fd >= 0 {
                c::close(self.fd);
            }
            if self.bt_capacity_fd >= 0 {
                c::close(self.bt_capacity_fd);
            }
            if self.bt_status_fd >= 0 {
                c::close(self.bt_status_fd);
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
    fn from_u16(btn: u16) -> Self {
        if btn >= BTN_SOUTH && btn <= BTN_THUMBR {
            unsafe { mem::transmute(btn - (BTN_SOUTH - constants::BTN_SOUTH)) }
        } else if btn >= BTN_DPAD_UP && btn <= BTN_DPAD_RIGHT {
            unsafe { mem::transmute(btn - (BTN_DPAD_UP - constants::BTN_DPAD_UP)) }
        } else {
            Button::Unknown
        }
    }
}

impl Axis {
    fn from_u16(axis: u16) -> Self {
        if axis >= ABS_X && axis <= ABS_RZ {
            unsafe { mem::transmute(axis) }
        } else if axis >= ABS_HAT1X && axis <= ABS_HAT2Y {
            unsafe { mem::transmute(axis - 10) }
        } else {
            Axis::Unknown
        }
    }
}

fn test_bit(n: u16, array: &[u8]) -> bool {
    (array[(n / 8) as usize] >> (n % 8)) & 1 != 0
}

unsafe fn cstr_new(bytes: &[u8]) -> &CStr {
    CStr::from_bytes_with_nul_unchecked(bytes)
}

const KEY_MAX: u16 = 0x2ff;
const EV_MAX: u16 = 0x1f;
const EV_KEY: u16 = 0x01;
const EV_ABS: u16 = 0x03;
const ABS_MAX: u16 = 0x3f;
const EV_FF: u16 = 0x15;

const BTN_MISC: u16 = 0x100;
const BTN_MOUSE: u16 = 0x110;
const BTN_JOYSTICK: u16 = 0x120;
const BTN_GAMEPAD: u16 = 0x130;
const BTN_SOUTH: u16 = 0x130;
const BTN_EAST: u16 = 0x131;
#[allow(dead_code)]
const BTN_C: u16 = 0x132;
const BTN_NORTH: u16 = 0x133;
const BTN_WEST: u16 = 0x134;
#[allow(dead_code)]
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
const ABS_Z: u16 = 0x02;
const ABS_RX: u16 = 0x03;
const ABS_RY: u16 = 0x04;
const ABS_RZ: u16 = 0x05;
const ABS_HAT0X: u16 = 0x10;
const ABS_HAT0Y: u16 = 0x11;
const ABS_HAT1X: u16 = 0x12;
const ABS_HAT1Y: u16 = 0x13;
const ABS_HAT2X: u16 = 0x14;
const ABS_HAT2Y: u16 = 0x15;

const FF_MAX: u16 = FF_GAIN;
const FF_SQUARE: u16 = 0x58;
const FF_TRIANGLE: u16 = 0x59;
const FF_SINE: u16 = 0x5a;
const FF_GAIN: u16 = 0x60;

pub mod native_ev_codes {
    pub const BTN_SOUTH: u16 = super::BTN_SOUTH;
    pub const BTN_EAST: u16 = super::BTN_EAST;
    #[allow(dead_code)]
    pub const BTN_C: u16 = super::BTN_C;
    pub const BTN_NORTH: u16 = super::BTN_NORTH;
    pub const BTN_WEST: u16 = super::BTN_WEST;
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pub const AXIS_LEFTZ: u16 = super::ABS_Z;
    pub const AXIS_RSTICKX: u16 = super::ABS_RX;
    pub const AXIS_RSTICKY: u16 = super::ABS_RY;
    #[allow(dead_code)]
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
    use super::create_uuid;
    use gamepad::{Button, Axis};

    #[test]
    fn sdl_uuid() {
        let x = Uuid::parse_str("030000005e0400008e02000020200000").unwrap();
        let y = create_uuid(ioctl::input_id {
            bustype: 0x3,
            vendor: 0x045e,
            product: 0x028e,
            version: 0x2020,
        });
        assert_eq!(x, y);
    }

    #[test]
    fn btn_from_u16() {
        assert_eq!(Some(Button::South), Button::from_u16(super::BTN_SOUTH));
        assert_eq!(Some(Button::East), Button::from_u16(super::BTN_EAST));
        assert_eq!(Some(Button::North), Button::from_u16(super::BTN_NORTH));
        assert_eq!(Some(Button::West), Button::from_u16(super::BTN_WEST));
        assert_eq!(Some(Button::C), Button::from_u16(super::BTN_C));
        assert_eq!(Some(Button::Z), Button::from_u16(super::BTN_Z));
        assert_eq!(Some(Button::LeftTrigger), Button::from_u16(super::BTN_TL));
        assert_eq!(Some(Button::LeftTrigger2), Button::from_u16(super::BTN_TL2));
        assert_eq!(Some(Button::RightTrigger), Button::from_u16(super::BTN_TR));
        assert_eq!(Some(Button::RightTrigger2),
                   Button::from_u16(super::BTN_TR2));
        assert_eq!(Some(Button::Select), Button::from_u16(super::BTN_SELECT));
        assert_eq!(Some(Button::Start), Button::from_u16(super::BTN_START));
        assert_eq!(Some(Button::Mode), Button::from_u16(super::BTN_MODE));
        assert_eq!(Some(Button::LeftThumb), Button::from_u16(super::BTN_THUMBL));
        assert_eq!(Some(Button::RightThumb),
                   Button::from_u16(super::BTN_THUMBR));
        assert_eq!(Some(Button::DPadUp), Button::from_u16(super::BTN_DPAD_UP));
        assert_eq!(Some(Button::DPadDown),
                   Button::from_u16(super::BTN_DPAD_DOWN));
        assert_eq!(Some(Button::DPadLeft),
                   Button::from_u16(super::BTN_DPAD_LEFT));
        assert_eq!(Some(Button::DPadRight),
                   Button::from_u16(super::BTN_DPAD_RIGHT));

        assert_eq!(None, Button::from_u16(super::BTN_SOUTH - 1));
        assert_eq!(None, Button::from_u16(super::BTN_THUMBR + 1));
        assert_eq!(None, Button::from_u16(super::BTN_DPAD_UP - 1));
        assert_eq!(None, Button::from_u16(super::BTN_DPAD_RIGHT + 1));
    }

    #[test]
    fn axis_from_u16() {
        assert_eq!(Some(Axis::LeftStickX), Axis::from_u16(super::ABS_X));
        assert_eq!(Some(Axis::LeftStickY), Axis::from_u16(super::ABS_Y));
        assert_eq!(Some(Axis::LeftZ), Axis::from_u16(super::ABS_Z));
        assert_eq!(Some(Axis::RightStickX), Axis::from_u16(super::ABS_RX));
        assert_eq!(Some(Axis::RightStickY), Axis::from_u16(super::ABS_RY));
        assert_eq!(Some(Axis::RightZ), Axis::from_u16(super::ABS_RZ));
        assert_eq!(Some(Axis::LeftTrigger), Axis::from_u16(super::ABS_HAT1Y));
        assert_eq!(Some(Axis::LeftTrigger2), Axis::from_u16(super::ABS_HAT2Y));
        assert_eq!(Some(Axis::RightTrigger), Axis::from_u16(super::ABS_HAT1X));
        assert_eq!(Some(Axis::RightTrigger2), Axis::from_u16(super::ABS_HAT2X));

        assert_eq!(None, Axis::from_u16(super::ABS_RZ + 1));
        assert_eq!(None, Axis::from_u16(super::ABS_HAT1X - 1));
        assert_eq!(None, Axis::from_u16(super::ABS_HAT2Y + 1));
    }
}
