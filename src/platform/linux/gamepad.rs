// Copyright 2017 Mateusz Sieczko and other GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use super::ff::Device as FfDevice;
use super::ioctl;
use super::ioctl::{input_absinfo, input_event};
use super::udev::*;
use AsInner;
use gamepad::{Axis, Button, Event, EventType, Gamepad as MainGamepad, GamepadImplExt,
              NativeEvCode, PowerInfo, Status};
use utils::test_bit;

use libc as c;
use uuid::Uuid;
use vec_map::VecMap;

use std::collections::VecDeque;
use std::ffi::CStr;
use std::mem;
use std::ops::Index;
use std::str;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub struct Gilrs {
    gamepads: Vec<MainGamepad>,
    monitor: Option<Monitor>,
    not_observed: MainGamepad,
    event_counter: usize,
    additional_events: VecDeque<Event>,
}

impl Gilrs {
    pub fn new() -> Self {
        let mut gamepads = Vec::new();
        let mut additional_events = VecDeque::new();

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
                if let Some(gamepad) = Gamepad::open(&dev) {
                    gamepads.push(MainGamepad::from_inner_status(gamepad, Status::Connected));
                    additional_events
                        .push_back(Event::new(gamepads.len() - 1, EventType::Connected));
                }
            }
        }

        let monitor = Monitor::new(&udev);
        if monitor.is_none() {
            error!("Failed to create udev monitor. Hotplugging will not be supported");
        }

        Gilrs {
            gamepads,
            monitor,
            not_observed: MainGamepad::from_inner_status(Gamepad::none(), Status::NotObserved),
            event_counter: 0,
            additional_events,
        }
    }

    fn none() -> Self {
        Gilrs {
            gamepads: Vec::new(),
            monitor: None,
            not_observed: MainGamepad::from_inner_status(Gamepad::none(), Status::NotObserved),
            event_counter: 0,
            additional_events: VecDeque::new(),
        }
    }

    pub fn next_event(&mut self) -> Option<Event> {
        if let Some(event) = self.additional_events.pop_front() {
            return Some(event);
        }

        if let Some(event) = self.handle_hotplug() {
            return Some(event);
        }

        loop {
            let gamepad = match self.gamepads.get_mut(self.event_counter) {
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
                Some((event, time)) => return Some(Event { id: self.event_counter, event, time }),
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

    fn handle_hotplug(&mut self) -> Option<Event> {
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
                    if let Some(gamepad) = Gamepad::open(&dev) {
                        if let Some(id) = self.gamepads.iter().position(|gp| {
                            gp.uuid() == gamepad.uuid && gp.status() == Status::Disconnected
                        }) {
                            self.gamepads[id] =
                                MainGamepad::from_inner_status(gamepad, Status::Connected);
                            return Some(Event::new(id, EventType::Connected));
                        } else {
                            self.gamepads
                                .push(MainGamepad::from_inner_status(gamepad, Status::Connected));
                            return Some(Event::new(self.gamepads.len() - 1, EventType::Connected));
                        }
                    }
                } else if action == cstr_new(b"remove\0") {
                    if let Some(devnode) = dev.devnode() {
                        if let Some(id) = self.gamepads.iter().position(|gp| {
                            is_eq_cstr_str(devnode, &gp.as_inner().devpath) && gp.is_connected()
                        }) {
                            self.gamepads[id].as_inner_mut().disconnect();
                            return Some(Event::new(id, EventType::Disconnected));
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
        if *l_ptr == 0 && r_ptr == end {
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct AxesInfo {
    info: VecMap<input_absinfo>,
}

impl AxesInfo {
    fn new(fd: i32) -> Self {
        let mut map = VecMap::new();
        unsafe {
            let mut abs_bits = [0u8; (ABS_MAX / 8) as usize + 1];
            ioctl::eviocgbit(
                fd,
                EV_ABS as u32,
                abs_bits.len() as i32,
                abs_bits.as_mut_ptr(),
            );
            for axis in Gamepad::find_axes(&abs_bits) {
                let mut info = input_absinfo::default();
                ioctl::eviocgabs(fd, axis as u32, &mut info);
                map.insert(axis as usize, info);
            }
        }
        AxesInfo { info: map }
    }

    fn deadzone(&self, idx: u16) -> f32 {
        self.info
            .get(idx as usize)
            .map_or(0.1, |i| i.flat as f32 / i.maximum as f32)
    }
}

impl Index<u16> for AxesInfo {
    type Output = input_absinfo;

    fn index(&self, i: u16) -> &Self::Output {
        &self.info[i as usize]
    }
}

#[derive(Debug)]
pub struct Gamepad {
    fd: i32,
    axes_info: AxesInfo,
    ff_supported: bool,
    devpath: String,
    name: String,
    uuid: Uuid,
    // TODO: path or RefCell<File>
    bt_capacity_fd: i32,
    // TODO: path or RefCell<File>
    bt_status_fd: i32,
    axes_values: VecMap<i32>,
    buttons_values: VecMap<bool>,
    dropped_events: Vec<input_event>,
    axes: Vec<u16>,
    buttons: Vec<u16>,
}

impl Gamepad {
    fn none() -> Self {
        Gamepad {
            fd: -3,
            axes_info: unsafe { mem::zeroed() },
            ff_supported: false,
            devpath: String::new(),
            name: String::new(),
            uuid: Uuid::nil(),
            bt_status_fd: -1,
            bt_capacity_fd: -1,
            axes_values: VecMap::new(),
            buttons_values: VecMap::new(),
            dropped_events: Vec::new(),
            axes: Vec::new(),
            buttons: Vec::new(),
        }
    }

    fn open(dev: &Device) -> Option<Gamepad> {
        let path = match dev.devnode() {
            Some(path) => path,
            None => return None,
        };

        if unsafe { !c::strstr(path.as_ptr(), b"js\0".as_ptr() as *const i8).is_null() } {
            info!("Device {:?} is js interface, ignoring.", path);
            return None;
        }

        let fd = unsafe { c::open(path.as_ptr(), c::O_RDWR | c::O_NONBLOCK) };
        if fd < 0 {
            error!("Failed to open {:?}", path);
            return None;
        }

        let uuid = match Self::create_uuid(fd) {
            Some(uuid) => uuid,
            None => {
                error!("Failed to get id of device {:?}", path);
                unsafe {
                    c::close(fd);
                }
                return None;
            }
        };


        let name = Self::get_name(fd).unwrap_or_else(|| {
            error!("Failed to get name od device {:?}", path);
            "Unknown".into()
        });

        let axesi = AxesInfo::new(fd);
        let ff_supported = Self::test_ff(fd);
        let (cap, status) = Self::battery_fd(&dev);

        let mut gamepad = Gamepad {
            fd: fd,
            axes_info: axesi,
            ff_supported: ff_supported,
            devpath: path.to_string_lossy().into_owned(),
            name: name,
            uuid: uuid,
            bt_capacity_fd: cap,
            bt_status_fd: status,
            axes_values: VecMap::new(),
            buttons_values: VecMap::new(),
            dropped_events: Vec::new(),
            axes: Vec::new(),
            buttons: Vec::new(),
        };

        gamepad.collect_axes_and_buttons();

        if !gamepad.is_gamepad() {
            warn!(
                "{:?} doesn't have at least 1 button and 2 axes, ignoring.",
                path
            );
            return None;
        }

        info!("Found {:#?}", gamepad);

        Some(gamepad)
    }

    fn collect_axes_and_buttons(&mut self) {
        let mut key_bits = [0u8; (KEY_MAX / 8) as usize + 1];
        let mut abs_bits = [0u8; (ABS_MAX / 8) as usize + 1];

        unsafe {
            ioctl::eviocgbit(
                self.fd,
                EV_KEY as u32,
                key_bits.len() as i32,
                key_bits.as_mut_ptr(),
            );
            ioctl::eviocgbit(
                self.fd,
                EV_ABS as u32,
                abs_bits.len() as i32,
                abs_bits.as_mut_ptr(),
            );
        }

        self.buttons = Self::find_buttons(&key_bits, false);
        self.axes = Self::find_axes(&abs_bits);
    }



    fn get_name(fd: i32) -> Option<String> {
        unsafe {
            let mut namebuff = mem::uninitialized::<[u8; 128]>();
            if ioctl::eviocgname(fd, &mut namebuff).is_err() {
                None
            } else {
                Some(
                    CStr::from_ptr(namebuff.as_ptr() as *const i8)
                        .to_string_lossy()
                        .into_owned(),
                )
            }
        }
    }

    fn test_ff(fd: i32) -> bool {
        unsafe {
            let mut ff_bits = [0u8; (FF_MAX / 8) as usize + 1];
            if ioctl::eviocgbit(fd, EV_FF as u32, ff_bits.len() as i32, ff_bits.as_mut_ptr()) >= 0 {
                if test_bit(FF_SQUARE, &ff_bits) && test_bit(FF_TRIANGLE, &ff_bits)
                    && test_bit(FF_SINE, &ff_bits) && test_bit(FF_GAIN, &ff_bits)
                {
                    true
                } else {
                    false
                }
            } else {
                false
            }
        }
    }

    fn is_gamepad(&self) -> bool {
        // TODO: improve it (for example check for buttons in range)
        if self.buttons.len() >= 1 && self.axes.len() >= 2 {
            true
        } else {
            false
        }
    }

    fn create_uuid(fd: i32) -> Option<Uuid> {
        let mut iid;
        unsafe {
            iid = mem::uninitialized::<ioctl::input_id>();
            if ioctl::eviocgid(fd, &mut iid).is_err() {
                return None;
            }
        }
        Some(create_uuid(iid))
    }

    fn find_buttons(key_bits: &[u8], only_gamepad_btns: bool) -> Vec<u16> {
        let mut buttons = Vec::with_capacity(16);

        for bit in BTN_MISC..BTN_MOUSE {
            if test_bit(bit, &key_bits) {
                buttons.push(bit);
            }
        }
        for bit in BTN_JOYSTICK..(key_bits.len() as u16 * 8) {
            if test_bit(bit, &key_bits) {
                buttons.push(bit);
            }
        }

        if !only_gamepad_btns {
            for bit in 0..BTN_MISC {
                if test_bit(bit, &key_bits) {
                    buttons.push(bit);
                }
            }
            for bit in BTN_MOUSE..BTN_JOYSTICK {
                if test_bit(bit, &key_bits) {
                    buttons.push(bit);
                }
            }
        }

        buttons
    }

    fn find_axes(abs_bits: &[u8]) -> Vec<u16> {
        let mut axes = Vec::with_capacity(8);

        for bit in 0..(abs_bits.len() * 8) {
            if test_bit(bit as u16, &abs_bits) {
                axes.push(bit as u16);
            }
        }

        axes
    }

    fn battery_fd(dev: &Device) -> (i32, i32) {
        use std::ffi::OsStr;
        use std::fs::{self, File};
        use std::os::unix::ffi::OsStrExt;
        use std::os::unix::io::IntoRawFd;
        use std::path::Path;

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

    pub fn event(&mut self) -> Option<(EventType, SystemTime)> {
        let mut skip = false;
        // Skip all unknown events and return Option on first know event or when there is no more
        // events to read. Returning None on unknown event breaks iterators.
        loop {
            let event = match self.next_event() {
                Some(e) => e,
                None => return None,
            };

            if skip {
                if event.type_ == EV_SYN && event.code == SYN_REPORT {
                    skip = false;
                    self.compare_state();
                }
                continue;
            }

            let ev = match event.type_ {
                EV_SYN if event.code == SYN_DROPPED => {
                    skip = true;
                    None
                }
                EV_KEY => {
                    self.buttons_values
                        .insert(event.code as usize, event.value == 1);
                    let btn = Button::Unknown;
                    match event.value {
                        0 => Some(EventType::ButtonReleased(btn, event.code)),
                        1 => Some(EventType::ButtonPressed(btn, event.code)),
                        _ => None,
                    }
                }
                EV_ABS => {
                    let axis_info = &self.axes_info[event.code];
                    self.axes_values.insert(event.code as usize, event.value);
                    let val = Self::axis_value(*axis_info, event.value, event.code);

                    Some(EventType::AxisChanged(Axis::Unknown, val, event.code))
                }
                _ => None,
            };

            if let Some(ev) = ev {
                let dur = Duration::new(event.time.tv_sec as u64, event.time.tv_usec as u32 * 1000);

                return Some((ev, UNIX_EPOCH + dur));
            }
        }
    }

    fn next_event(&mut self) -> Option<input_event> {
        if self.dropped_events.len() > 0 {
            self.dropped_events.pop()
        } else {
            unsafe {
                let mut event = mem::uninitialized::<ioctl::input_event>();
                let size = mem::size_of::<ioctl::input_event>();
                let n = c::read(self.fd, mem::transmute(&mut event), size);

                if n == -1 || n == 0 {
                    // Nothing to read (non-blocking IO)
                    return None;
                } else if n != size as isize {
                    unreachable!()
                }

                Some(event)
            }
        }
    }

    fn compare_state(&mut self) {
        for axis in self.axes.iter().cloned() {
            let value = unsafe {
                let mut absinfo = mem::uninitialized();
                ioctl::eviocgabs(self.fd, axis as u32, &mut absinfo);
                absinfo.value
            };

            if self.axes_values.get(axis as usize).cloned().unwrap_or(0) != value {
                self.dropped_events.push(input_event {
                    type_: EV_ABS,
                    code: axis,
                    value: value,
                    ..Default::default()
                });
            }
        }

        let mut buf = [0u8; KEY_MAX as usize / 8 + 1];
        unsafe {
            let _ = ioctl::eviocgkey(self.fd, &mut buf);
        }

        for btn in self.buttons.iter().cloned() {
            let val = test_bit(btn, &buf);
            if self.buttons_values
                .get(btn as usize)
                .cloned()
                .unwrap_or(false) != val
            {
                self.dropped_events.push(input_event {
                    type_: EV_KEY,
                    code: btn,
                    value: val as i32,
                    ..Default::default()
                });
            }
        }
    }

    fn axis_value(axes_info: input_absinfo, val: i32, axis: u16) -> f32 {
        let mut val =
            val as f32 / if val < 0 { -axes_info.minimum } else { axes_info.maximum } as f32;
        // FIXME: axis is not mapped
        if (axis == ABS_X || axis == ABS_Y || axis == ABS_RX || axis == ABS_RY || axis == ABS_Z
            || axis == ABS_RZ) && axes_info.minimum == 0
        {
            val = (val - 0.5) * 2.0
        }
        val * if axis == ABS_Y || axis == ABS_RY || axis == ABS_RZ || axis == ABS_HAT0Y {
            -1.0
        } else {
            1.0
        }
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

                let len = c::read(
                    self.bt_capacity_fd,
                    mem::transmute(buff.as_mut_ptr()),
                    buff.len(),
                ) as usize;

                if len > 0 {
                    let cap = match str::from_utf8_unchecked(&buff[..(len - 1)]).parse() {
                        Ok(cap) => cap,
                        Err(_) => {
                            error!(
                                "Failed to parse battery capacity: {}",
                                str::from_utf8_unchecked(&buff[..(len - 1)])
                            );
                            return PowerInfo::Unknown;
                        }
                    };

                    let len = c::read(
                        self.bt_status_fd,
                        mem::transmute(buff.as_mut_ptr()),
                        buff.len(),
                    ) as usize;

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
            if self.fd > -1 {
                PowerInfo::Wired
            } else {
                PowerInfo::Unknown
            }
        }
    }

    pub fn is_ff_supported(&self) -> bool {
        self.ff_supported
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_owned()
    }

    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    pub fn ff_device(&self) -> Option<FfDevice> {
        if self.is_ff_supported() {
            FfDevice::new(&self.devpath).ok()
        } else {
            None
        }
    }

    pub fn buttons(&self) -> &[NativeEvCode] {
        &self.buttons
    }

    pub fn axes(&self) -> &[NativeEvCode] {
        &self.axes
    }

    pub fn deadzone(&self, axis: NativeEvCode) -> f32 {
        self.axes_info.deadzone(axis)
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
    Uuid::from_fields(
        bus,
        vendor,
        0,
        &[
            (product >> 8) as u8, product as u8, 0, 0, (version >> 8) as u8, version as u8, 0, 0
        ],
    ).unwrap()
}

unsafe fn cstr_new(bytes: &[u8]) -> &CStr {
    CStr::from_bytes_with_nul_unchecked(bytes)
}

const KEY_MAX: u16 = 0x2ff;
#[allow(dead_code)]
const EV_MAX: u16 = 0x1f;
const EV_SYN: u16 = 0x00;
const EV_KEY: u16 = 0x01;
const EV_ABS: u16 = 0x03;
const ABS_MAX: u16 = 0x3f;
const EV_FF: u16 = 0x15;

const SYN_REPORT: u16 = 0x00;
const SYN_DROPPED: u16 = 0x03;

const BTN_MISC: u16 = 0x100;
const BTN_MOUSE: u16 = 0x110;
const BTN_JOYSTICK: u16 = 0x120;
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
    use super::create_uuid;
    use super::super::ioctl;
    use uuid::Uuid;

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
}
