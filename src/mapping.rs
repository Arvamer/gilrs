use vec_map::VecMap;
use std::collections::HashMap;
use platform;
use platform::native_ev_codes;
use std::env;
use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};
use uuid::{Uuid, ParseError as UuidError};

#[derive(Debug)]
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
                return Err(ParseSdlMappingError::MissingValue);
            }

            match key {
                "platform" => {
                    if val != platform::NAME {
                        return Err(ParseSdlMappingError::NotTargetPlatform);
                    }
                }
                "x" => {
                    let code = try!(Mapping::get_btn(val, buttons));
                    mapping.btns.insert(code as usize, native_ev_codes::BTN_EAST);
                }
                "a" => {
                    let code = try!(Mapping::get_btn(val, buttons));
                    mapping.btns.insert(code as usize, native_ev_codes::BTN_SOUTH);
                }
                "b" => {
                    let code = try!(Mapping::get_btn(val, buttons));
                    mapping.btns.insert(code as usize, native_ev_codes::BTN_WEST);
                }
                "y" => {
                    let code = try!(Mapping::get_btn(val, buttons));
                    mapping.btns.insert(code as usize, native_ev_codes::BTN_NORTH);
                }
                "back" => {
                    let code = try!(Mapping::get_btn(val, buttons));
                    mapping.btns.insert(code as usize, native_ev_codes::BTN_SELECT);
                }
                "guide" => {
                    let code = try!(Mapping::get_btn(val, buttons));
                    mapping.btns.insert(code as usize, native_ev_codes::BTN_MODE);
                }
                "start" => {
                    let code = try!(Mapping::get_btn(val, buttons));
                    mapping.btns.insert(code as usize, native_ev_codes::BTN_START);
                }
                "leftstick" => {
                    let code = try!(Mapping::get_btn(val, buttons));
                    mapping.btns.insert(code as usize, native_ev_codes::BTN_LTHUMB);
                }
                "rightstick" => {
                    let code = try!(Mapping::get_btn(val, buttons));
                    mapping.btns.insert(code as usize, native_ev_codes::BTN_RTHUMB);
                }
                "leftx" => {
                    let code = try!(Mapping::get_axis(val, axes));
                    mapping.axes.insert(code as usize, native_ev_codes::AXIS_LSTICKX);
                }
                "lefty" => {
                    let code = try!(Mapping::get_axis(val, axes));
                    mapping.axes.insert(code as usize, native_ev_codes::AXIS_LSTICKY);
                }
                "rightx" => {
                    let code = try!(Mapping::get_axis(val, axes));
                    mapping.axes.insert(code as usize, native_ev_codes::AXIS_RSTICKX);
                }
                "righty" => {
                    let code = try!(Mapping::get_axis(val, axes));
                    mapping.axes.insert(code as usize, native_ev_codes::AXIS_RSTICKY);
                }
                "leftshoulder" => {
                    match try!(Mapping::get_btn_or_axis(val, buttons, axes)) {
                        BtnOrAxis::Axis(val) => {
                            mapping.axes.insert(val as usize, native_ev_codes::AXIS_LT)
                        }
                        BtnOrAxis::Button(val) => {
                            mapping.btns.insert(val as usize, native_ev_codes::BTN_LT)
                        }
                    };
                }
                "lefttrigger" => {
                    match try!(Mapping::get_btn_or_axis(val, buttons, axes)) {
                        BtnOrAxis::Axis(val) => {
                            mapping.axes.insert(val as usize, native_ev_codes::AXIS_LT2)
                        }
                        BtnOrAxis::Button(val) => {
                            mapping.btns.insert(val as usize, native_ev_codes::BTN_LT2)
                        }
                    };
                }
                "rightshoulder" => {
                    match try!(Mapping::get_btn_or_axis(val, buttons, axes)) {
                        BtnOrAxis::Axis(val) => {
                            mapping.axes.insert(val as usize, native_ev_codes::AXIS_RT)
                        }
                        BtnOrAxis::Button(val) => {
                            mapping.btns.insert(val as usize, native_ev_codes::BTN_RT)
                        }
                    };
                }
                "righttrigger" => {
                    match try!(Mapping::get_btn_or_axis(val, buttons, axes)) {
                        BtnOrAxis::Axis(val) => {
                            mapping.axes.insert(val as usize, native_ev_codes::AXIS_RT2)
                        }
                        BtnOrAxis::Button(val) => {
                            mapping.btns.insert(val as usize, native_ev_codes::BTN_RT2)
                        }
                    };
                }
                "dpleft" => {
                    match try!(Mapping::get_btn_or_axis(val, buttons, axes)) {
                        BtnOrAxis::Axis(val) => {
                            mapping.axes.insert(val as usize, native_ev_codes::AXIS_DPADX)
                        }
                        BtnOrAxis::Button(val) => {
                            mapping.btns.insert(val as usize, native_ev_codes::BTN_DPAD_LEFT)
                        }
                    };
                }
                "dpright" => {
                    match try!(Mapping::get_btn_or_axis(val, buttons, axes)) {
                        BtnOrAxis::Axis(val) => {
                            mapping.axes.insert(val as usize, native_ev_codes::AXIS_DPADX)
                        }
                        BtnOrAxis::Button(val) => {
                            mapping.btns.insert(val as usize, native_ev_codes::BTN_DPAD_RIGHT)
                        }
                    };
                }
                "dpup" => {
                    match try!(Mapping::get_btn_or_axis(val, buttons, axes)) {
                        BtnOrAxis::Axis(val) => {
                            mapping.axes.insert(val as usize, native_ev_codes::AXIS_DPADY)
                        }
                        BtnOrAxis::Button(val) => {
                            mapping.btns.insert(val as usize, native_ev_codes::BTN_DPAD_UP)
                        }
                    };
                }
                "dpdown" => {
                    match try!(Mapping::get_btn_or_axis(val, buttons, axes)) {
                        BtnOrAxis::Axis(val) => {
                            mapping.axes.insert(val as usize, native_ev_codes::AXIS_DPADY)
                        }
                        BtnOrAxis::Button(val) => {
                            mapping.btns.insert(val as usize, native_ev_codes::BTN_DPAD_DOWN)
                        }
                    };
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
        buttons.get(val).cloned().ok_or(ParseSdlMappingError::InvalidValue)
    }

    fn get_axis(val: &str, axes: &[u16]) -> Result<u16, ParseSdlMappingError> {
        let (ident, val) = val.split_at(1);
        if ident == "a" {
            let val = match val.parse() {
                Ok(val) => val,
                Err(_) => return Err(ParseSdlMappingError::InvalidValue),
            };
            axes.get(val).cloned().ok_or(ParseSdlMappingError::InvalidValue)
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
                1 | 4 => Ok(platform::native_ev_codes::AXIS_DPADY),
                2 | 8 => Ok(platform::native_ev_codes::AXIS_DPADX),
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
    MissingValue,
}

impl ParseSdlMappingError {
    fn to_str(self) -> &'static str {
        match self {
            ParseSdlMappingError::MissingGuid => "GUID is missing",
            ParseSdlMappingError::InvalidGuid => "GUID is invalid",
            ParseSdlMappingError::MissingName => "device name is missing",
            ParseSdlMappingError::InvalidPair => "key-value pair is invalid",
            ParseSdlMappingError::NotTargetPlatform => "mapping for different OS than target",
            ParseSdlMappingError::InvalidValue => "value is invalid",
            ParseSdlMappingError::MissingValue => "value is missing",
        }
    }
}

impl Error for ParseSdlMappingError {
    fn description(&self) -> &str {
        self.to_str()
    }
}

impl Display for ParseSdlMappingError {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        fmt.write_str(self.to_str())
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
        let mut hmap = HashMap::new();

        Self::insert_to(include_str!("../SDL_GameControllerDB/gamecontrollerdb.txt"), &mut hmap);

        if let Ok(mapping) = env::var("SDL_GAMECONTROLLERCONFIG") {
            Self::insert_to(&mapping, &mut hmap);
        }

        /*
        let mut c = 0;
        for s in hmap.values() {
            if Mapping::parse_sdl_mapping(s, &[0u16; 16], &[0u16; 16]).is_ok() {
                c += 1;
            }
        }
        println!("{}", c);
        */
        MappingDb { mappings: hmap }
    }

    fn insert_to(s: &str, map: &mut HashMap<Uuid, String>) {
        for mapping in s.lines() {
            mapping.split(',')
                   .next()
                   .and_then(|s| Uuid::parse_str(s).ok())
                   .and_then(|uuid| map.insert(uuid, mapping.to_owned()));
        }
    }

    pub fn get(&self, uuid: Uuid) -> Option<&String> {
        self.mappings.get(&uuid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Do not include platform, mapping from
    // https://github.com/gabomdq/SDL_GameControllerDB/blob/master/gamecontrollerdb.txt
    const TEST_STR: &'static str = "03000000260900008888000000010000,GameCube {WiseGroup USB \
                                    box},a:b0,b:b2,y:b3,x:b1,start:b7,rightshoulder:b6,dpup:h0.1,\
                                    dpleft:h0.8,dpdown:h0.4,dpright:h0.2,leftx:a0,lefty:a1,rightx:\
                                    a2,righty:a3,lefttrigger:a4,righttrigger:a5,";

    const BUTTONS: [u16; 12] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
    const AXES: [u16; 8] = [0, 1, 2, 3, 4, 5, 6, 7];

    #[test]
    fn mapping() {
        let _ = Mapping::parse_sdl_mapping(TEST_STR, &BUTTONS, &AXES).unwrap();
    }
}
