// Copyright 2016 GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
#![cfg_attr(target_os = "windows", allow(dead_code))]

use vec_map::VecMap;
use std::collections::HashMap;
use std::ops::{Index, IndexMut};
use platform::{self, native_ev_codes as nec};
use gamepad::{NativeEvCode, Axis, Button};
use std::env;
use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};
use uuid::{Uuid, ParseError as UuidError};

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
/// Store mappings from one `NativeEvCode` (`u16`) to another.
///
/// This struct is internal, `MappingData` is exported in public interface as `Mapping`.
pub struct Mapping {
    axes: VecMap<u16>,
    btns: VecMap<u16>,
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

    pub fn from_data(data: &MappingData,
                     buttons: &[u16],
                     axes: &[u16],
                     name: &str,
                     uuid: Uuid)
                     -> Result<(Self, String), MappingError> {
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
           data.buttons.contains_key(Button::RightTrigger2 as usize) {
            return Err(MappingError::DuplicatedEntry);
        }

        let mut mapped_btns = VecMap::<u16>::new();
        let mut mapped_axes = VecMap::<u16>::new();
        let mut sdl_mappings = format!("{},{},", uuid.simple(), name);

        {
            let mut add_button = |ident, ev_code, mapped_ev_code| {
                Self::add_button(ident,
                                 ev_code,
                                 mapped_ev_code,
                                 buttons,
                                 &mut sdl_mappings,
                                 &mut mapped_btns)
            };

            for (button, &ev_code) in &data.buttons {
                match button as u16 {
                    BTN_SOUTH => add_button("a", ev_code, nec::BTN_SOUTH)?,
                    BTN_EAST => add_button("b", ev_code, nec::BTN_EAST)?,
                    BTN_WEST => add_button("x", ev_code, nec::BTN_WEST)?,
                    BTN_NORTH => add_button("y", ev_code, nec::BTN_NORTH)?,
                    BTN_LT => add_button("leftshoulder", ev_code, nec::BTN_LT)?,
                    BTN_RT => add_button("rightshoulder", ev_code, nec::BTN_RT)?,
                    BTN_LT2 => add_button("lefttrigger", ev_code, nec::BTN_LT2)?,
                    BTN_RT2 => add_button("righttrigger", ev_code, nec::BTN_RT2)?,
                    BTN_SELECT => add_button("back", ev_code, nec::BTN_SELECT)?,
                    BTN_START => add_button("start", ev_code, nec::BTN_START)?,
                    BTN_MODE => add_button("guide", ev_code, nec::BTN_MODE)?,
                    BTN_LTHUMB => add_button("leftstick", ev_code, nec::BTN_LTHUMB)?,
                    BTN_RTHUMB => add_button("rightstick", ev_code, nec::BTN_RTHUMB)?,
                    BTN_DPAD_UP => add_button("dpup", ev_code, nec::BTN_DPAD_UP)?,
                    BTN_DPAD_DOWN => add_button("dpdown", ev_code, nec::BTN_DPAD_DOWN)?,
                    BTN_DPAD_LEFT => add_button("dpleft", ev_code, nec::BTN_DPAD_LEFT)?,
                    BTN_DPAD_RIGHT => add_button("dpright", ev_code, nec::BTN_DPAD_RIGHT)?,
                    _ => unreachable!(),
                }
            }
        }

        {
            let mut add_axis = |ident, ev_code, mapped_ev_code| {
                Self::add_axis(ident,
                               ev_code,
                               mapped_ev_code,
                               axes,
                               &mut sdl_mappings,
                               &mut mapped_axes)
            };

            for (axis, &ev_code) in &data.axes {
                match axis as u16 {
                    AXIS_LSTICKX => add_axis("leftx", ev_code, nec::AXIS_LSTICKX)?,
                    AXIS_LSTICKY => add_axis("lefty", ev_code, nec::AXIS_LSTICKY)?,
                    AXIS_RSTICKX => add_axis("rightx", ev_code, nec::AXIS_RSTICKX)?,
                    AXIS_RSTICKY => add_axis("righty", ev_code, nec::AXIS_RSTICKY)?,
                    AXIS_RT => add_axis("rightshoulder", ev_code, nec::AXIS_RT)?,
                    AXIS_LT => add_axis("leftshoulder", ev_code, nec::AXIS_LT)?,
                    AXIS_RT2 => add_axis("righttrigger", ev_code, nec::AXIS_RT2)?,
                    AXIS_LT2 => add_axis("lefttrigger", ev_code, nec::AXIS_LT2)?,
                    _ => unreachable!(),
                }
            }
        }

        let mapping = Mapping {
            axes: mapped_axes,
            btns: mapped_btns,
            name: name.to_owned(),
        };

        Ok((mapping, sdl_mappings))
    }

    pub fn parse_sdl_mapping(line: &str,
                             buttons: &[u16],
                             axes: &[u16])
                             -> Result<Self, ParseSdlMappingError> {
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

            let key = match pair.next() {
                Some(key) => key,
                None => return Err(ParseSdlMappingError::InvalidPair),
            };
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
                    try!(Mapping::insert_btn(val, buttons, m_btns, nec::BTN_WEST));
                }
                "a" => {
                    try!(Mapping::insert_btn(val, buttons, m_btns, nec::BTN_SOUTH));
                }
                "b" => {
                    try!(Mapping::insert_btn(val, buttons, m_btns, nec::BTN_EAST));
                }
                "y" => {
                    try!(Mapping::insert_btn(val, buttons, m_btns, nec::BTN_NORTH));
                }
                "back" => {
                    try!(Mapping::insert_btn(val, buttons, m_btns, nec::BTN_SELECT));
                }
                "guide" => {
                    try!(Mapping::insert_btn(val, buttons, m_btns, nec::BTN_MODE));
                }
                "start" => {
                    try!(Mapping::insert_btn(val, buttons, m_btns, nec::BTN_START));
                }
                "leftstick" => {
                    try!(Mapping::insert_btn(val, buttons, m_btns, nec::BTN_LTHUMB));
                }
                "rightstick" => {
                    try!(Mapping::insert_btn(val, buttons, m_btns, nec::BTN_RTHUMB));
                }
                "leftx" => {
                    try!(Mapping::insert_axis(val, axes, m_axes, nec::AXIS_LSTICKX));
                }
                "lefty" => {
                    try!(Mapping::insert_axis(val, axes, m_axes, nec::AXIS_LSTICKY));
                }
                "rightx" => {
                    try!(Mapping::insert_axis(val, axes, m_axes, nec::AXIS_RSTICKX));
                }
                "righty" => {
                    try!(Mapping::insert_axis(val, axes, m_axes, nec::AXIS_RSTICKY));
                }
                "leftshoulder" => {
                    try!(Mapping::insert_btn_or_axis(val,
                                                     buttons,
                                                     axes,
                                                     m_btns,
                                                     m_axes,
                                                     nec::BTN_LT,
                                                     nec::AXIS_LT));
                }
                "lefttrigger" => {
                    try!(Mapping::insert_btn_or_axis(val,
                                                     buttons,
                                                     axes,
                                                     m_btns,
                                                     m_axes,
                                                     nec::BTN_LT2,
                                                     nec::AXIS_LT2));
                }
                "rightshoulder" => {
                    try!(Mapping::insert_btn_or_axis(val,
                                                     buttons,
                                                     axes,
                                                     m_btns,
                                                     m_axes,
                                                     nec::BTN_RT,
                                                     nec::AXIS_RT));
                }
                "righttrigger" => {
                    try!(Mapping::insert_btn_or_axis(val,
                                                     buttons,
                                                     axes,
                                                     m_btns,
                                                     m_axes,
                                                     nec::BTN_RT2,
                                                     nec::AXIS_RT2));
                }
                "dpleft" => {
                    try!(Mapping::insert_btn_or_axis(val,
                                                     buttons,
                                                     axes,
                                                     m_btns,
                                                     m_axes,
                                                     nec::BTN_DPAD_LEFT,
                                                     nec::AXIS_DPADX));
                }
                "dpright" => {
                    try!(Mapping::insert_btn_or_axis(val,
                                                     buttons,
                                                     axes,
                                                     m_btns,
                                                     m_axes,
                                                     nec::BTN_DPAD_RIGHT,
                                                     nec::AXIS_DPADX));
                }
                "dpup" => {
                    try!(Mapping::insert_btn_or_axis(val,
                                                     buttons,
                                                     axes,
                                                     m_btns,
                                                     m_axes,
                                                     nec::BTN_DPAD_UP,
                                                     nec::AXIS_DPADY));
                }
                "dpdown" => {
                    try!(Mapping::insert_btn_or_axis(val,
                                                     buttons,
                                                     axes,
                                                     m_btns,
                                                     m_axes,
                                                     nec::BTN_DPAD_DOWN,
                                                     nec::AXIS_DPADY));
                }
                _ => (),
            }
        }

        Ok(mapping)
    }

    fn get_btn(val: &str, buttons: &[u16]) -> Result<u16, ParseSdlMappingError> {
        let (ident, val) = val.split_at(1);
        if ident != "b" {
            return Err(ParseSdlMappingError::InvalidValue);
        }
        let val = match val.parse() {
            Ok(val) => val,
            Err(_) => return Err(ParseSdlMappingError::InvalidValue),
        };
        buttons.get(val).cloned().ok_or(ParseSdlMappingError::InvalidBtn)
    }

    fn get_axis(val: &str, axes: &[u16]) -> Result<u16, ParseSdlMappingError> {
        let (ident, val) = val.split_at(1);
        if ident == "a" {
            let val = match val.parse() {
                Ok(val) => val,
                Err(_) => return Err(ParseSdlMappingError::InvalidValue),
            };
            axes.get(val).cloned().ok_or(ParseSdlMappingError::InvalidAxis)
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

    fn get_btn_or_axis(val: &str,
                       buttons: &[u16],
                       axes: &[u16])
                       -> Result<BtnOrAxis, ParseSdlMappingError> {
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

    fn insert_btn(s: &str,
                  btns: &[u16],
                  map: &mut VecMap<u16>,
                  ncode: u16)
                  -> Result<(), ParseSdlMappingError> {
        match Mapping::get_btn(s, btns) {
            Ok(code) => {
                map.insert(code as usize, ncode);
            }
            Err(ParseSdlMappingError::InvalidBtn) => (),
            Err(e) => return Err(e),
        };
        Ok(())
    }

    fn insert_axis(s: &str,
                   axes: &[u16],
                   map: &mut VecMap<u16>,
                   ncode: u16)
                   -> Result<(), ParseSdlMappingError> {
        match Mapping::get_axis(s, axes) {
            Ok(code) => {
                map.insert(code as usize, ncode);
            }
            Err(ParseSdlMappingError::InvalidAxis) => (),
            Err(e) => return Err(e),
        };
        Ok(())
    }

    fn insert_btn_or_axis(s: &str,
                          btns: &[u16],
                          axes: &[u16],
                          map_btns: &mut VecMap<u16>,
                          map_axes: &mut VecMap<u16>,
                          ncode_btn: u16,
                          ncode_axis: u16)
                          -> Result<(), ParseSdlMappingError> {
        match Mapping::get_btn_or_axis(s, btns, axes) {
            Ok(BtnOrAxis::Button(code)) => {
                map_btns.insert(code as usize, ncode_btn);
            }
            Ok(BtnOrAxis::Axis(code)) => {
                map_axes.insert(code as usize, ncode_axis);
            }
            Err(ParseSdlMappingError::InvalidAxis) => (),
            Err(e) => return Err(e),
        };
        Ok(())
    }

    fn add_button(ident: &str,
                  ev_code: u16,
                  mapped_ev_code: u16,
                  buttons: &[u16],
                  sdl_mappings: &mut String,
                  mapped_btns: &mut VecMap<u16>)
                  -> Result<(), MappingError> {
        let n_btn =
            buttons.iter().position(|&x| x == ev_code).ok_or(MappingError::InvalidCode(ev_code))?;
        sdl_mappings.push_str(&format!("{}:b{},", ident, n_btn));
        mapped_btns.insert(ev_code as usize, mapped_ev_code);
        Ok(())
    }

    fn add_axis(ident: &str,
                ev_code: u16,
                mapped_ev_code: u16,
                axes: &[u16],
                sdl_mappings: &mut String,
                mapped_axes: &mut VecMap<u16>)
                -> Result<(), MappingError> {
        let n_axis =
            axes.iter().position(|&x| x == ev_code).ok_or(MappingError::InvalidCode(ev_code))?;
        sdl_mappings.push_str(&format!("{}:a{},", ident, n_axis));
        mapped_axes.insert(ev_code as usize, mapped_ev_code);
        Ok(())
    }

    fn is_name_valid(name: &str) -> bool {
        !name.chars().any(|x| x == ',')
    }

    pub fn map(&self, code: u16, kind: Kind) -> u16 {
        match kind {
            Kind::Button => *self.btns.get(code as usize).unwrap_or(&code),
            Kind::Axis => *self.axes.get(code as usize).unwrap_or(&code),
        }
    }

    pub fn map_rev(&self, code: u16, kind: Kind) -> u16 {
        match kind {
            Kind::Button => {
                self.btns
                    .iter()
                    .find(|x| *x.1 == code)
                    .unwrap_or((code as usize, &0))
                    .0 as u16
            }
            Kind::Axis => {
                self.axes.iter().find(|x| *x.1 == code).unwrap_or((code as usize, &0)).0 as u16
            }
        }
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

pub enum Kind {
    Button,
    Axis,
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
            mapping.split(',')
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
    use gamepad::{Button, Axis};
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
        let buttons = [10, 11, 12, 13, 14, 15];
        let axes = [0, 1, 2, 3];

        let mut data = MappingData::new();
        data[Axis::LeftStickX] = 0;
        data[Axis::LeftStickY] = 1;
        data[Axis::LeftTrigger] = 2;
        data[Axis::LeftTrigger2] = 3;
        data[Button::South] = 10;
        data[Button::South] = 10;
        data[Button::West] = 11;
        data[Button::Start] = 15;

        let (mappings, sdl_mappings) = Mapping::from_data(&data, &buttons, &axes, name, uuid)
            .unwrap();
        let sdl_mappings = Mapping::parse_sdl_mapping(&sdl_mappings, &buttons, &axes).unwrap();
        assert_eq!(mappings, sdl_mappings);

        let incorrect_mappings = Mapping::from_data(&data, &buttons, &axes, "Inval,id name", uuid);
        assert_eq!(Err(MappingError::InvalidName), incorrect_mappings);

        data[Button::South] = 22;
        let incorrect_mappings = Mapping::from_data(&data, &buttons, &axes, name, uuid);
        assert_eq!(Err(MappingError::InvalidCode(22)), incorrect_mappings);

        data[Button::South] = 10;
        data[Button::LeftTrigger] = 11;
        let incorrect_mappings = Mapping::from_data(&data, &buttons, &axes, name, uuid);
        assert_eq!(Err(MappingError::DuplicatedEntry), incorrect_mappings);
    }

    #[test]
    fn with_mappings() {
        let mappings = format!("\nShould be ignored\nThis also should,be ignored\n\n{}", TEST_STR);
        let db = MappingDb::with_mappings(&mappings);
        assert_eq!(Some(TEST_STR),
                   db.get(Uuid::parse_str("03000000260900008888000000010001").unwrap()));
    }
}
