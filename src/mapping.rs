// Copyright 2016 GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
#![cfg_attr(target_os = "windows", allow(dead_code))]

use gamepad::{Axis, Button, NativeEvCode};
use platform::{self, native_ev_codes as nec};
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::ops::{Index, IndexMut};
use uuid::{ParseError as UuidError, Uuid};
use vec_map::VecMap;

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
/// Store mappings from one `NativeEvCode` (`u16`) to another.
///
/// This struct is internal, `MappingData` is exported in public interface as `Mapping`.
pub struct Mapping {
    axes: VecMap<Axis>,
    btns: VecMap<Button>,
    name: String,
}

impl Mapping {
    pub fn new() -> Self {
        Mapping {
            axes: VecMap::new(),
            btns: VecMap::new(),
            name: String::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn from_data(
        data: &MappingData,
        buttons: &[u16],
        axes: &[u16],
        name: &str,
        uuid: Uuid,
    ) -> Result<(Self, String), MappingError> {
        use constants::*;

        if !Self::is_name_valid(name) {
            return Err(MappingError::InvalidName);
        }

        if data.axes.contains_key(Axis::LeftTrigger as usize) &&
            data.buttons.contains_key(Button::LeftTrigger as usize) ||
            data.axes.contains_key(Axis::LeftTrigger2 as usize) &&
                data.buttons.contains_key(Button::LeftTrigger2 as usize) ||
            data.axes.contains_key(Axis::RightTrigger as usize) &&
                data.buttons.contains_key(Button::RightTrigger as usize) ||
            data.axes.contains_key(Axis::RightTrigger2 as usize) &&
                data.buttons.contains_key(Button::RightTrigger2 as usize)
        {
            return Err(MappingError::DuplicatedEntry);
        }

        let mut mapped_btns = VecMap::<Button>::new();
        let mut mapped_axes = VecMap::<Axis>::new();
        let mut sdl_mappings = format!("{},{},", uuid.simple(), name);

        {
            let mut add_button = |ident, ev_code, mapped_btn| {
                Self::add_button(
                    ident,
                    ev_code,
                    mapped_btn,
                    buttons,
                    &mut sdl_mappings,
                    &mut mapped_btns,
                )
            };

            for (button, &ev_code) in &data.buttons {
                match button as u16 {
                    BTN_SOUTH => add_button("a", ev_code, Button::South)?,
                    BTN_EAST => add_button("b", ev_code, Button::East)?,
                    BTN_WEST => add_button("x", ev_code, Button::West)?,
                    BTN_NORTH => add_button("y", ev_code, Button::North)?,
                    BTN_LT => add_button("leftshoulder", ev_code, Button::LeftTrigger)?,
                    BTN_RT => add_button("rightshoulder", ev_code, Button::RightTrigger)?,
                    BTN_LT2 => add_button("lefttrigger", ev_code, Button::LeftTrigger2)?,
                    BTN_RT2 => add_button("righttrigger", ev_code, Button::RightTrigger2)?,
                    BTN_SELECT => add_button("back", ev_code, Button::Select)?,
                    BTN_START => add_button("start", ev_code, Button::Start)?,
                    BTN_MODE => add_button("guide", ev_code, Button::Mode)?,
                    BTN_LTHUMB => add_button("leftstick", ev_code, Button::LeftThumb)?,
                    BTN_RTHUMB => add_button("rightstick", ev_code, Button::RightThumb)?,
                    BTN_DPAD_UP => add_button("dpup", ev_code, Button::DPadUp)?,
                    BTN_DPAD_DOWN => add_button("dpdown", ev_code, Button::DPadDown)?,
                    BTN_DPAD_LEFT => add_button("dpleft", ev_code, Button::DPadLeft)?,
                    BTN_DPAD_RIGHT => add_button("dpright", ev_code, Button::DPadRight)?,
                    BTN_C => add_button("c", ev_code, Button::C)?,
                    BTN_Z => add_button("z", ev_code, Button::Z)?,
                    BTN_UNKNOWN => return Err(MappingError::UnknownElement),
                    _ => unreachable!(),
                }
            }
        }

        {
            let mut add_axis = |ident, ev_code, mapped_axis| {
                Self::add_axis(
                    ident,
                    ev_code,
                    mapped_axis,
                    axes,
                    &mut sdl_mappings,
                    &mut mapped_axes,
                )
            };

            for (axis, &ev_code) in &data.axes {
                match axis as u16 {
                    AXIS_LSTICKX => add_axis("leftx", ev_code, Axis::LeftStickX)?,
                    AXIS_LSTICKY => add_axis("lefty", ev_code, Axis::LeftStickY)?,
                    AXIS_RSTICKX => add_axis("rightx", ev_code, Axis::RightStickX)?,
                    AXIS_RSTICKY => add_axis("righty", ev_code, Axis::RightStickY)?,
                    AXIS_RT => add_axis("rightshoulder", ev_code, Axis::RightTrigger)?,
                    AXIS_LT => add_axis("leftshoulder", ev_code, Axis::LeftTrigger)?,
                    AXIS_RT2 => add_axis("righttrigger", ev_code, Axis::RightTrigger2)?,
                    AXIS_LT2 => add_axis("lefttrigger", ev_code, Axis::LeftTrigger2)?,
                    AXIS_LEFTZ => add_axis("leftz", ev_code, Axis::LeftZ)?,
                    AXIS_RIGHTZ => add_axis("rightz", ev_code, Axis::RightZ)?,
                    AXIS_UNKNOWN => return Err(MappingError::UnknownElement),
                    _ => unreachable!(),
                }
            }
        }

        let mut mapping = Mapping {
            axes: mapped_axes,
            btns: mapped_btns,
            name: name.to_owned(),
        };

        mapping.unmap_not_mapped_axes();

        Ok((mapping, sdl_mappings))
    }

    pub fn parse_sdl_mapping(
        line: &str,
        buttons: &[NativeEvCode],
        axes: &[NativeEvCode],
    ) -> Result<Self, ParseSdlMappingError> {
        let mut parts = line.split(',');

        let _ = match parts.next() {
            Some(uuid) => uuid,
            None => return Err(ParseSdlMappingError::MissingGuid),
        };

        let name = match parts.next() {
            Some(name) => name,
            None => return Err(ParseSdlMappingError::MissingName),
        };

        let mut mapping = Mapping::new();
        mapping.name = name.to_owned();

        for pair in parts {
            let mut pair = pair.split(':');

            let key = pair.next().ok_or(ParseSdlMappingError::InvalidPair)?;

            let val = match pair.next() {
                Some(val) => val,
                None => continue,
            };

            if val.is_empty() {
                continue;
            }

            let m_btns = &mut mapping.btns;
            let m_axes = &mut mapping.axes;

            match key {
                "platform" => {
                    if val != platform::NAME {
                        return Err(ParseSdlMappingError::NotTargetPlatform);
                    }
                }
                "x" => {
                    Mapping::insert_btn(val, buttons, m_btns, Button::West)?;
                }
                "a" => {
                    Mapping::insert_btn(val, buttons, m_btns, Button::South)?;
                }
                "b" => {
                    Mapping::insert_btn(val, buttons, m_btns, Button::East)?;
                }
                "y" => {
                    Mapping::insert_btn(val, buttons, m_btns, Button::North)?;
                }
                "c" => {
                    Mapping::insert_btn(val, buttons, m_btns, Button::C)?;
                }
                "z" => {
                    Mapping::insert_btn(val, buttons, m_btns, Button::Z)?;
                }
                "back" => {
                    Mapping::insert_btn(val, buttons, m_btns, Button::Select)?;
                }
                "guide" => {
                    Mapping::insert_btn(val, buttons, m_btns, Button::Mode)?;
                }
                "start" => {
                    Mapping::insert_btn(val, buttons, m_btns, Button::Start)?;
                }
                "leftstick" => {
                    Mapping::insert_btn(val, buttons, m_btns, Button::LeftThumb)?;
                }
                "rightstick" => {
                    Mapping::insert_btn(val, buttons, m_btns, Button::RightThumb)?;
                }
                "leftx" => {
                    Mapping::insert_axis(val, axes, m_axes, Axis::LeftStickX)?;
                }
                "lefty" => {
                    Mapping::insert_axis(val, axes, m_axes, Axis::LeftStickY)?;
                }
                "rightx" => {
                    Mapping::insert_axis(val, axes, m_axes, Axis::RightStickX)?;
                }
                "righty" => {
                    Mapping::insert_axis(val, axes, m_axes, Axis::RightStickY)?;
                }
                "leftz" => {
                    Mapping::insert_axis(val, axes, m_axes, Axis::LeftZ)?;
                }
                "rightz" => {
                    Mapping::insert_axis(val, axes, m_axes, Axis::RightZ)?;
                }
                "leftshoulder" => {
                    Mapping::insert_btn_or_axis(
                        val,
                        buttons,
                        axes,
                        m_btns,
                        m_axes,
                        Button::LeftTrigger,
                        Axis::LeftTrigger,
                    )?;
                }
                "lefttrigger" => {
                    Mapping::insert_btn_or_axis(
                        val,
                        buttons,
                        axes,
                        m_btns,
                        m_axes,
                        Button::LeftTrigger2,
                        Axis::LeftTrigger2,
                    )?;
                }
                "rightshoulder" => {
                    Mapping::insert_btn_or_axis(
                        val,
                        buttons,
                        axes,
                        m_btns,
                        m_axes,
                        Button::RightTrigger,
                        Axis::RightTrigger,
                    )?;
                }
                "righttrigger" => {
                    Mapping::insert_btn_or_axis(
                        val,
                        buttons,
                        axes,
                        m_btns,
                        m_axes,
                        Button::RightTrigger2,
                        Axis::RightTrigger2,
                    )?;
                }
                "dpleft" => {
                    Mapping::insert_btn_or_axis(
                        val,
                        buttons,
                        axes,
                        m_btns,
                        m_axes,
                        Button::DPadLeft,
                        Axis::DPadX,
                    )?;
                }
                "dpright" => {
                    Mapping::insert_btn_or_axis(
                        val,
                        buttons,
                        axes,
                        m_btns,
                        m_axes,
                        Button::DPadRight,
                        Axis::DPadX,
                    )?;
                }
                "dpup" => {
                    Mapping::insert_btn_or_axis(
                        val,
                        buttons,
                        axes,
                        m_btns,
                        m_axes,
                        Button::DPadUp,
                        Axis::DPadY,
                    )?;
                }
                "dpdown" => {
                    Mapping::insert_btn_or_axis(
                        val,
                        buttons,
                        axes,
                        m_btns,
                        m_axes,
                        Button::DPadDown,
                        Axis::DPadY,
                    )?;
                }
                _ => (),
            }
        }

        mapping.unmap_not_mapped_axes();

        Ok(mapping)
    }

    fn get_btn(val: &str, buttons: &[NativeEvCode]) -> Result<NativeEvCode, ParseSdlMappingError> {
        let (ident, val) = val.split_at(1);
        if ident != "b" {
            return Err(ParseSdlMappingError::InvalidValue);
        }
        let val = match val.parse::<usize>() {
            Ok(val) => val,
            Err(_) => return Err(ParseSdlMappingError::InvalidValue),
        };
        buttons.get(val).cloned().ok_or(
            ParseSdlMappingError::InvalidBtn,
        )
    }

    fn get_axis(val: &str, axes: &[NativeEvCode]) -> Result<NativeEvCode, ParseSdlMappingError> {
        let (ident, val) = val.split_at(1);
        if ident == "a" {
            let val = match val.parse::<usize>() {
                Ok(val) => val,
                Err(_) => return Err(ParseSdlMappingError::InvalidValue),
            };
            axes.get(val).cloned().ok_or(
                ParseSdlMappingError::InvalidAxis,
            )
        } else if ident == "h" {
            let mut val_it = val.split('.');

            match val_it.next().and_then(|s| s.parse::<u16>().ok()) {
                Some(hat) if hat == 0 => hat,
                _ => return Err(ParseSdlMappingError::InvalidValue),
            };

            let dir = match val_it.next().and_then(|s| s.parse().ok()) {
                Some(dir) => dir,
                None => return Err(ParseSdlMappingError::InvalidValue),
            };

            match dir {
                1 | 4 => Ok(nec::AXIS_DPADY),
                2 | 8 => Ok(nec::AXIS_DPADX),
                _ => Err(ParseSdlMappingError::InvalidValue),
            }
        } else {
            Err(ParseSdlMappingError::InvalidValue)
        }
    }

    fn get_btn_or_axis(
        val: &str,
        buttons: &[NativeEvCode],
        axes: &[NativeEvCode],
    ) -> Result<BtnOrAxis, ParseSdlMappingError> {
        if let Some(c) = val.as_bytes().get(0) {
            match *c as char {
                'a' | 'h' => Mapping::get_axis(val, axes).and_then(|val| Ok(BtnOrAxis::Axis(val))),
                'b' => Mapping::get_btn(val, buttons).and_then(|val| Ok(BtnOrAxis::Button(val))),
                _ => Err(ParseSdlMappingError::InvalidValue),
            }
        } else {
            Err(ParseSdlMappingError::InvalidValue)
        }
    }

    fn insert_btn(
        s: &str,
        btns: &[NativeEvCode],
        map: &mut VecMap<Button>,
        btn: Button,
    ) -> Result<(), ParseSdlMappingError> {
        match Mapping::get_btn(s, btns) {
            Ok(code) => {
                map.insert(code as usize, btn);
            }
            Err(ParseSdlMappingError::InvalidBtn) => (),
            Err(e) => return Err(e),
        };
        Ok(())
    }

    fn insert_axis(
        s: &str,
        axes: &[NativeEvCode],
        map: &mut VecMap<Axis>,
        axis: Axis,
    ) -> Result<(), ParseSdlMappingError> {
        match Mapping::get_axis(s, axes) {
            Ok(code) => {
                map.insert(code as usize, axis);
            }
            Err(ParseSdlMappingError::InvalidAxis) => (),
            Err(e) => return Err(e),
        };
        Ok(())
    }

    fn insert_btn_or_axis(
        s: &str,
        btns: &[u16],
        axes: &[u16],
        map_btns: &mut VecMap<Button>,
        map_axes: &mut VecMap<Axis>,
        btn: Button,
        axis: Axis,
    ) -> Result<(), ParseSdlMappingError> {
        match Mapping::get_btn_or_axis(s, btns, axes) {
            Ok(BtnOrAxis::Button(code)) => {
                map_btns.insert(code as usize, btn);
            }
            Ok(BtnOrAxis::Axis(code)) => {
                map_axes.insert(code as usize, axis);
            }
            Err(ParseSdlMappingError::InvalidAxis) => (),
            Err(e) => return Err(e),
        };
        Ok(())
    }

    fn add_button(
        ident: &str,
        ev_code: u16,
        mapped_btn: Button,
        buttons: &[u16],
        sdl_mappings: &mut String,
        mapped_btns: &mut VecMap<Button>,
    ) -> Result<(), MappingError> {
        let n_btn = buttons.iter().position(|&x| x == ev_code).ok_or(
            MappingError::InvalidCode(ev_code),
        )?;
        sdl_mappings.push_str(&format!("{}:b{},", ident, n_btn));
        mapped_btns.insert(ev_code as usize, mapped_btn);
        Ok(())
    }

    fn add_axis(
        ident: &str,
        ev_code: u16,
        mapped_axis: Axis,
        axes: &[u16],
        sdl_mappings: &mut String,
        mapped_axes: &mut VecMap<Axis>,
    ) -> Result<(), MappingError> {
        let n_axis = axes.iter().position(|&x| x == ev_code).ok_or(
            MappingError::InvalidCode(ev_code),
        )?;
        sdl_mappings.push_str(&format!("{}:a{},", ident, n_axis));
        mapped_axes.insert(ev_code as usize, mapped_axis);
        Ok(())
    }

    fn is_name_valid(name: &str) -> bool {
        !name.chars().any(|x| x == ',')
    }

    pub fn map_button(&self, code: NativeEvCode) -> Button {
        self.btns.get(code as usize).cloned().unwrap_or(
            Button::Unknown,
        )
    }

    pub fn map_axis(&self, code: NativeEvCode) -> Axis {
        self.axes.get(code as usize).cloned().unwrap_or(
            Axis::Unknown,
        )
    }

    pub fn map_rev_axis(&self, axis: Axis) -> Option<NativeEvCode> {
        self.axes.iter().find(|x| *x.1 == axis).map(|x| {
            x.0 as NativeEvCode
        })
    }

    pub fn map_rev_button(&self, btn: Button) -> Option<NativeEvCode> {
        self.btns.iter().find(|x| *x.1 == btn).map(|x| {
            x.0 as NativeEvCode
        })
    }

    fn unmap_not_mapped_axes(&mut self) {
        let mut mapped_axes = self.axes
            .iter()
            .filter(|&(from, &to)| from != to as usize)
            .map(|(_, &to)| to as u16)
            .collect::<Vec<_>>();
        mapped_axes.sort();
        mapped_axes.dedup();
        for mapped_axis in mapped_axes.into_iter() {
            self.axes.entry(mapped_axis as usize).or_insert(
                Axis::Unknown,
            );
        }
    }
}

impl Default for Mapping {
    fn default() -> Self {
        macro_rules! vec_map {
            ( $( $key:expr => $elem:expr ),* ) => {
                {
                    let mut map = VecMap::new();
                    $(
                        map.insert($key as usize, $elem);
                    )*

                    map
                }
            };
        }

        let btns =
            vec_map![
            nec::BTN_SOUTH => Button::South,
            nec::BTN_EAST => Button::East,
            nec::BTN_C => Button::C,
            nec::BTN_NORTH => Button::North,
            nec::BTN_WEST => Button::West,
            nec::BTN_Z => Button::Z,
            nec::BTN_LT => Button::LeftTrigger,
            nec::BTN_RT => Button::RightTrigger,
            nec::BTN_LT2 => Button::LeftTrigger2,
            nec::BTN_RT2 => Button::RightTrigger2,
            nec::BTN_SELECT => Button::Select,
            nec::BTN_START => Button::Start,
            nec::BTN_MODE => Button::Mode,
            nec::BTN_LTHUMB => Button::LeftThumb,
            nec::BTN_RTHUMB => Button::RightThumb,
            nec::BTN_DPAD_UP => Button::DPadUp,
            nec::BTN_DPAD_DOWN => Button::DPadDown,
            nec::BTN_DPAD_LEFT => Button::DPadLeft,
            nec::BTN_DPAD_RIGHT => Button::DPadRight
        ];

        let axes =
            vec_map![
            nec::AXIS_LSTICKX => Axis::LeftStickX,
            nec::AXIS_LSTICKY => Axis::LeftStickY,
            nec::AXIS_LEFTZ => Axis::LeftZ,
            nec::AXIS_RSTICKX => Axis::RightStickX,
            nec::AXIS_RSTICKY => Axis::RightStickY,
            nec::AXIS_RIGHTZ => Axis::RightZ,
            nec::AXIS_DPADX => Axis::DPadX,
            nec::AXIS_DPADY => Axis::DPadY,
            nec::AXIS_RT => Axis::RightTrigger,
            nec::AXIS_LT => Axis::LeftTrigger,
            nec::AXIS_RT2 => Axis::RightTrigger2,
            nec::AXIS_LT2 => Axis::LeftTrigger2
        ];

        Mapping { axes, btns, name: String::new() }
    }
}

enum BtnOrAxis {
    Axis(u16),
    Button(u16),
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum ParseSdlMappingError {
    MissingGuid,
    InvalidGuid,
    MissingName,
    InvalidPair,
    NotTargetPlatform,
    InvalidValue,
    InvalidBtn,
    InvalidAxis,
}

impl ParseSdlMappingError {
    fn into_str(self) -> &'static str {
        match self {
            ParseSdlMappingError::MissingGuid => "GUID is missing",
            ParseSdlMappingError::InvalidGuid => "GUID is invalid",
            ParseSdlMappingError::MissingName => "device name is missing",
            ParseSdlMappingError::InvalidPair => "key-value pair is invalid",
            ParseSdlMappingError::NotTargetPlatform => "mapping for different OS than target",
            ParseSdlMappingError::InvalidValue => "value is invalid",
            ParseSdlMappingError::InvalidBtn => "gamepad doesn't have requested button",
            ParseSdlMappingError::InvalidAxis => "gamepad doesn't have requested axis",
        }
    }
}

impl Error for ParseSdlMappingError {
    fn description(&self) -> &str {
        self.into_str()
    }
}

impl Display for ParseSdlMappingError {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        fmt.write_str(self.into_str())
    }
}

impl From<UuidError> for ParseSdlMappingError {
    fn from(_: UuidError) -> Self {
        ParseSdlMappingError::InvalidGuid
    }
}

#[derive(Debug)]
pub struct MappingDb {
    mappings: HashMap<Uuid, String>,
}

impl MappingDb {
    pub fn new() -> Self {
        Self::with_mappings("")
    }

    pub fn with_mappings(sdl_mappings: &str) -> Self {
        let mut db = MappingDb { mappings: HashMap::new() };

        db.insert(include_str!("../SDL_GameControllerDB/gamecontrollerdb.txt"));
        db.insert(sdl_mappings);

        if let Ok(mapping) = env::var("SDL_GAMECONTROLLERCONFIG") {
            db.insert(&mapping);
        }

        db
    }

    pub fn insert(&mut self, s: &str) {
        for mapping in s.lines() {
            mapping
                .split(',')
                .next()
                .and_then(|s| Uuid::parse_str(s).ok())
                .and_then(|uuid| self.mappings.insert(uuid, mapping.to_owned()));
        }
    }

    pub fn get(&self, uuid: Uuid) -> Option<&str> {
        self.mappings.get(&uuid).map(String::as_ref)
    }
}

/// Stores data used to map gamepad buttons and axes.
///
/// To add new element, you should use `IndexMut` operator using `Axis` or `Button` as index (see
/// example). After you add all mappings, use
/// [`Gamepad::set_mapping(…)`](struct.Gamepad.html#method.set_mapping) to change mapping of
/// existing gamepad.
///
/// Example
/// =======
///
/// ```
/// use gilrs::{Mapping, Button, Axis};
///
/// let mut data = Mapping::new();
/// // map native event code 3 to Axis::LeftStickX
/// data[Axis::LeftStickX] = 3;
/// // map native event code 3 to Button::South (although both are 3,
/// // they refer to different things)
/// data[Button::South] = 3;
///
/// assert_eq!(data.axis(Axis::LeftStickX), Some(3));
/// assert_eq!(data.button(Button::South), Some(3));
/// ```
///
/// See `examples/mapping.rs` for more detailed example.
#[derive(Debug, Clone)]
// Re-exported as Mapping
pub struct MappingData {
    buttons: VecMap<u16>,
    axes: VecMap<u16>,
}

impl MappingData {
    /// Creates new `Mapping`.
    pub fn new() -> Self {
        MappingData {
            buttons: VecMap::with_capacity(18),
            axes: VecMap::with_capacity(11),
        }
    }

    /// Returns `NativeEvCode` associated with button index.
    pub fn button(&self, idx: Button) -> Option<NativeEvCode> {
        self.buttons.get(idx as usize).cloned()
    }

    /// Returns `NativeEvCode` associated with axis index.
    pub fn axis(&self, idx: Axis) -> Option<NativeEvCode> {
        self.axes.get(idx as usize).cloned()
    }

    /// Removes button and returns associated `NativEvCode`.
    pub fn remove_button(&mut self, idx: Button) -> Option<NativeEvCode> {
        self.buttons.remove(idx as usize)
    }

    /// Removes axis and returns associated `NativEvCode`.
    pub fn remove_axis(&mut self, idx: Axis) -> Option<NativeEvCode> {
        self.axes.remove(idx as usize)
    }
}

impl Index<Button> for MappingData {
    type Output = NativeEvCode;

    fn index(&self, index: Button) -> &Self::Output {
        &self.buttons[index as usize]
    }
}

impl Index<Axis> for MappingData {
    type Output = NativeEvCode;

    fn index(&self, index: Axis) -> &Self::Output {
        &self.axes[index as usize]
    }
}

impl IndexMut<Button> for MappingData {
    fn index_mut(&mut self, index: Button) -> &mut Self::Output {
        self.buttons.entry(index as usize).or_insert(0)
    }
}

impl IndexMut<Axis> for MappingData {
    fn index_mut(&mut self, index: Axis) -> &mut Self::Output {
        self.axes.entry(index as usize).or_insert(0)
    }
}

/// The error type for functions related to gamepad mapping.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum MappingError {
    /// Gamepad does not have element referenced by `NativeEvCode`.
    InvalidCode(NativeEvCode),
    /// Name contains comma (',').
    InvalidName,
    /// This function is not implemented for current platform.
    NotImplemented,
    /// Gamepad is not connected.
    NotConnected,
    /// Same gamepad element is referenced by axis and button.
    DuplicatedEntry,
    /// `Mapping` with `Button::Unknown` or `Axis::Unknown`.
    UnknownElement,
    /// `Mapping` have button or axis that are not present in SDL2.
    NotSdl2Compatible,
}

impl MappingError {
    fn into_str(self) -> &'static str {
        match self {
            MappingError::InvalidCode(_) => {
                "gamepad does not have element with requested event code"
            }
            MappingError::InvalidName => "name can not contain comma",
            MappingError::NotImplemented => {
                "current platform does not implement setting custom mappings"
            }
            MappingError::NotConnected => "gamepad is not connected",
            MappingError::DuplicatedEntry => {
                "same gamepad element is referenced by axis and button"
            }
            MappingError::UnknownElement => "Button::Unknown and Axis::Unknown are not allowed",
            MappingError::NotSdl2Compatible => "one of buttons or axes is not compatible with SDL2",
        }
    }
}

impl Error for MappingError {
    fn description(&self) -> &str {
        self.into_str()
    }
}

impl Display for MappingError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        f.write_str(self.into_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gamepad::{Axis, Button};
    use uuid::Uuid;
    // Do not include platform, mapping from (with UUID modified)
    // https://github.com/gabomdq/SDL_GameControllerDB/blob/master/gamecontrollerdb.txt
    const TEST_STR: &'static str = "03000000260900008888000000010001,GameCube {WiseGroup USB \
                                    box},a:b0,b:b2,y:b3,x:b1,start:b7,rightshoulder:b6,dpup:h0.1,\
                                    dpleft:h0.8,dpdown:h0.4,dpright:h0.2,leftx:a0,lefty:a1,rightx:\
                                    a2,righty:a3,lefttrigger:a4,righttrigger:a5,";

    const BUTTONS: [u16; 12] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
    const AXES: [u16; 8] = [0, 1, 2, 3, 4, 5, 6, 7];

    #[test]
    fn mapping() {
        Mapping::parse_sdl_mapping(TEST_STR, &BUTTONS, &AXES).unwrap();
    }

    #[test]
    fn from_data() {
        let uuid = Uuid::nil();
        let name = "Best Gamepad";
        let buttons = [10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22];
        let axes = [0, 1, 2, 3, 4, 5, 6, 7];

        let mut data = MappingData::new();
        data[Axis::LeftStickX] = 0;
        data[Axis::LeftStickY] = 1;
        data[Axis::LeftTrigger] = 2;
        data[Axis::LeftTrigger2] = 3;
        data[Axis::RightTrigger] = 4;
        data[Axis::RightTrigger2] = 5;
        data[Axis::RightStickX] = 6;
        data[Axis::RightStickY] = 7;

        data[Button::South] = 10;
        data[Button::South] = 10;
        data[Button::East] = 11;
        data[Button::North] = 12;
        data[Button::West] = 13;
        data[Button::Select] = 14;
        data[Button::Start] = 15;
        data[Button::Mode] = 16;
        data[Button::DPadUp] = 17;
        data[Button::DPadDown] = 18;
        data[Button::DPadLeft] = 19;
        data[Button::DPadRight] = 20;
        data[Button::LeftThumb] = 21;
        data[Button::RightThumb] = 22;

        let (mappings, sdl_mappings) = Mapping::from_data(&data, &buttons, &axes, name, uuid)
            .unwrap();
        let sdl_mappings = Mapping::parse_sdl_mapping(&sdl_mappings, &buttons, &axes).unwrap();
        assert_eq!(mappings, sdl_mappings);

        data[Button::North] = data.button(Button::South).unwrap();
        let (mappings, sdl_mappings) = Mapping::from_data(&data, &buttons, &axes, name, uuid)
            .unwrap();
        let sdl_mappings = Mapping::parse_sdl_mapping(&sdl_mappings, &buttons, &axes).unwrap();
        assert_eq!(mappings, sdl_mappings);

        let incorrect_mappings = Mapping::from_data(&data, &buttons, &axes, "Inval,id name", uuid);
        assert_eq!(Err(MappingError::InvalidName), incorrect_mappings);

        data[Button::South] = 32;
        let incorrect_mappings = Mapping::from_data(&data, &buttons, &axes, name, uuid);
        assert_eq!(Err(MappingError::InvalidCode(32)), incorrect_mappings);

        data[Button::South] = 10;
        data[Button::LeftTrigger] = 11;
        let incorrect_mappings = Mapping::from_data(&data, &buttons, &axes, name, uuid);
        assert_eq!(Err(MappingError::DuplicatedEntry), incorrect_mappings);
    }

    #[test]
    fn from_data_not_sdl2() {
        let uuid = Uuid::nil();
        let name = "Best Gamepad";
        let buttons = [10, 11, 12, 13, 14, 15];
        let axes = [0, 1, 2, 3];

        let mut data = MappingData::new();
        data[Axis::LeftZ] = 0;
        data[Axis::RightZ] = 1;
        data[Button::C] = 10;
        data[Button::Z] = 11;

        let (mappings, sdl_mappings) = Mapping::from_data(&data, &buttons, &axes, name, uuid)
            .unwrap();
        let sdl_mappings = Mapping::parse_sdl_mapping(&sdl_mappings, &buttons, &axes).unwrap();
        assert_eq!(mappings, sdl_mappings);

        data[Button::Unknown] = 13;
        let incorrect_mappings = Mapping::from_data(&data, &buttons, &axes, name, uuid);
        assert_eq!(Err(MappingError::UnknownElement), incorrect_mappings);

        assert_eq!(data.remove_button(Button::Unknown), Some(13));
        data[Axis::Unknown] = 3;
        let incorrect_mappings = Mapping::from_data(&data, &buttons, &axes, name, uuid);
        assert_eq!(Err(MappingError::UnknownElement), incorrect_mappings);
    }

    #[test]
    fn with_mappings() {
        let mappings = format!(
            "\nShould be ignored\nThis also should,be ignored\n\n{}",
            TEST_STR
        );
        let db = MappingDb::with_mappings(&mappings);
        assert_eq!(
            Some(TEST_STR),
            db.get(Uuid::parse_str("03000000260900008888000000010001").unwrap())
        );
    }
}
