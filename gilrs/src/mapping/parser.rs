// Copyright 2016-2018 Mateusz Sieczko and other GilRs Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
use std::error::Error as StdError;
use std::fmt::{self, Display};

use uuid::Uuid;

use crate::ev::{Axis, AxisOrBtn, Button};

// Must be sorted!
static AXES_SDL: [&str; 31] = [
    "a",
    "b",
    "back",
    "c",
    "dpdown",
    "dpleft",
    "dpright",
    "dpup",
    "guide",
    "leftshoulder",
    "leftstick",
    "lefttrigger",
    "leftx",
    "lefty",
    "leftz",
    "misc1",
    "paddle1",
    "paddle2",
    "paddle3",
    "paddle4",
    "rightshoulder",
    "rightstick",
    "righttrigger",
    "rightx",
    "righty",
    "rightz",
    "start",
    "touchpad",
    "x",
    "y",
    "z",
];
static AXES: [AxisOrBtn; 31] = [
    AxisOrBtn::Btn(Button::South),
    AxisOrBtn::Btn(Button::East),
    AxisOrBtn::Btn(Button::Select),
    AxisOrBtn::Btn(Button::C),
    AxisOrBtn::Btn(Button::DPadDown),
    AxisOrBtn::Btn(Button::DPadLeft),
    AxisOrBtn::Btn(Button::DPadRight),
    AxisOrBtn::Btn(Button::DPadUp),
    AxisOrBtn::Btn(Button::Mode),
    AxisOrBtn::Btn(Button::LeftTrigger),
    AxisOrBtn::Btn(Button::LeftThumb),
    AxisOrBtn::Btn(Button::LeftTrigger2),
    AxisOrBtn::Axis(Axis::LeftStickX),
    AxisOrBtn::Axis(Axis::LeftStickY),
    AxisOrBtn::Axis(Axis::LeftZ),
    AxisOrBtn::Btn(Button::Unknown),
    AxisOrBtn::Btn(Button::Unknown),
    AxisOrBtn::Btn(Button::Unknown),
    AxisOrBtn::Btn(Button::Unknown),
    AxisOrBtn::Btn(Button::Unknown),
    AxisOrBtn::Btn(Button::RightTrigger),
    AxisOrBtn::Btn(Button::RightThumb),
    AxisOrBtn::Btn(Button::RightTrigger2),
    AxisOrBtn::Axis(Axis::RightStickX),
    AxisOrBtn::Axis(Axis::RightStickY),
    AxisOrBtn::Axis(Axis::RightZ),
    AxisOrBtn::Btn(Button::Start),
    AxisOrBtn::Btn(Button::Unknown),
    AxisOrBtn::Btn(Button::West),
    AxisOrBtn::Btn(Button::North),
    AxisOrBtn::Btn(Button::Z),
];

pub struct Parser<'a> {
    data: &'a str,
    pos: usize,
    state: State,
}

impl<'a> Parser<'a> {
    pub fn new(mapping: &'a str) -> Self {
        Parser {
            data: mapping,
            pos: 0,
            state: State::Uuid,
        }
    }

    pub fn next_token(&mut self) -> Option<Result<Token<'_>, Error>> {
        if self.pos >= self.data.len() {
            None
        } else {
            Some(match self.state {
                State::Uuid => self.parse_uuid(),
                State::Name => self.parse_name(),
                State::KeyVal => self.parse_key_val(),
                State::Invalid => Err(Error::new(ErrorKind::InvalidParserState, self.pos)),
            })
        }
    }

    fn parse_uuid(&mut self) -> Result<Token<'_>, Error> {
        let next_comma = self.next_comma_or_end();
        let uuid_field = &self.data[self.pos..next_comma];
        let uuid = if uuid_field == "xinput" {
            Ok(Token::Uuid(Uuid::nil()))
        } else {
            Uuid::parse_str(uuid_field)
                .map(Token::Uuid)
                .map_err(|_| Error::new(ErrorKind::InvalidGuid, self.pos))
        };

        if uuid.is_err() {
            self.state = State::Invalid;
        } else if next_comma == self.data.len() {
            self.state = State::Invalid;

            return Err(Error::new(ErrorKind::UnexpectedEnd, self.pos));
        } else {
            self.state = State::Name;
            self.pos = next_comma + 1;
        }

        uuid
    }

    fn parse_name(&mut self) -> Result<Token<'_>, Error> {
        let next_comma = self.next_comma_or_end();
        let name = &self.data[self.pos..next_comma];

        self.state = State::KeyVal;
        self.pos = next_comma + 1;

        Ok(Token::Name(name))
    }

    fn parse_key_val(&mut self) -> Result<Token<'_>, Error> {
        let next_comma = self.next_comma_or_end();
        let pair = &self.data[self.pos..next_comma];
        let pos = self.pos;
        self.pos = next_comma + 1;

        let mut split = pair.split(':');
        let key = split
            .next()
            .ok_or_else(|| Error::new(ErrorKind::InvalidKeyValPair, pos))?;
        let value = split
            .next()
            .ok_or_else(|| Error::new(ErrorKind::InvalidKeyValPair, pos))?;

        if split.next().is_some() {
            return Err(Error::new(ErrorKind::InvalidKeyValPair, pos));
        }

        if value.is_empty() {
            return Err(Error::new(ErrorKind::EmptyValue, pos));
        }

        if key == "platform" {
            return Ok(Token::Platform(value));
        }

        let mut input = AxisRange::Full;
        let mut output = AxisRange::Full;
        let mut inverted = false;
        let mut is_axis = false;

        let key = match key.get(0..1) {
            Some("+") => {
                output = AxisRange::UpperHalf;
                &key[1..]
            }
            Some("-") => {
                output = AxisRange::LowerHalf;
                &key[1..]
            }
            _ => key,
        };

        let from = match value.get(0..1) {
            Some("+") if value.get(1..2) == Some("a") => {
                is_axis = true;
                input = AxisRange::UpperHalf;

                if value.get((value.len() - 1)..) == Some("~") {
                    inverted = true;

                    &value[2..(value.len() - 1)]
                } else {
                    &value[2..]
                }
            }
            Some("-") if value.get(1..2) == Some("a") => {
                is_axis = true;
                input = AxisRange::LowerHalf;

                if value.get((value.len() - 1)..) == Some("~") {
                    inverted = true;

                    &value[2..(value.len() - 1)]
                } else {
                    &value[2..]
                }
            }
            Some("a") => {
                is_axis = true;

                if value.get((value.len() - 1)..) == Some("~") {
                    inverted = true;

                    &value[1..(value.len() - 1)]
                } else {
                    &value[1..]
                }
            }
            Some("b") => &value[1..],
            Some("h") => {
                let dot_idx = value
                    .find('.')
                    .ok_or_else(|| Error::new(ErrorKind::InvalidValue, pos))?;
                let hat = value[1..dot_idx]
                    .parse()
                    .map_err(|_| Error::new(ErrorKind::InvalidValue, pos + 1))?;
                let direction = value
                    .get((dot_idx + 1)..)
                    .and_then(|s| s.parse().ok())
                    .ok_or_else(|| Error::new(ErrorKind::InvalidValue, pos + dot_idx + 1))?;

                let idx = AXES_SDL
                    .binary_search(&key)
                    .map_err(|_| Error::new(ErrorKind::UnknownButton, pos))?;

                return Ok(Token::HatMapping {
                    hat,
                    direction,
                    to: AXES[idx],
                    output,
                });
            }
            _ => return Err(Error::new(ErrorKind::InvalidValue, pos)),
        }
        .parse::<u16>()
        .map_err(|_| Error::new(ErrorKind::InvalidValue, pos))?;

        if is_axis {
            let idx = AXES_SDL
                .binary_search(&key)
                .map_err(|_| Error::new(ErrorKind::UnknownAxis, pos))?;

            Ok(Token::AxisMapping {
                from,
                to: AXES[idx],
                input,
                output,
                inverted,
            })
        } else {
            let idx = AXES_SDL
                .binary_search(&key)
                .map_err(|_| Error::new(ErrorKind::UnknownButton, pos))?;

            Ok(Token::ButtonMapping {
                from,
                to: AXES[idx],
                output,
            })
        }
    }

    fn next_comma_or_end(&self) -> usize {
        self.data[self.pos..]
            .find(',')
            .map(|x| x + self.pos)
            .unwrap_or_else(|| self.data.len())
    }
}

#[derive(Debug)]
pub enum Token<'a> {
    Uuid(Uuid),
    Platform(&'a str),
    Name(&'a str),
    AxisMapping {
        from: u16,
        to: AxisOrBtn,
        input: AxisRange,
        output: AxisRange,
        inverted: bool,
    },
    ButtonMapping {
        from: u16,
        to: AxisOrBtn,
        output: AxisRange,
    },
    // This is just SDL representation, we will convert this to axis mapping later
    HatMapping {
        hat: u16,
        // ?
        direction: u16,
        to: AxisOrBtn,
        output: AxisRange,
    },
}

#[repr(u8)]
#[derive(Debug)]
pub enum AxisRange {
    LowerHalf,
    UpperHalf,
    Full,
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum State {
    Uuid,
    Name,
    KeyVal,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub(crate) position: usize,
    kind: ErrorKind,
}

impl Error {
    pub fn new(kind: ErrorKind, position: usize) -> Self {
        Error { position, kind }
    }

    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    InvalidGuid,
    InvalidKeyValPair,
    InvalidValue,
    EmptyValue,
    UnknownAxis,
    UnknownButton,
    InvalidParserState,
    UnexpectedEnd,
}

impl StdError for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self.kind {
            ErrorKind::InvalidGuid => "GUID is invalid",
            ErrorKind::InvalidKeyValPair => "expected key value pair",
            ErrorKind::InvalidValue => "value is not valid",
            ErrorKind::EmptyValue => "value is empty",
            ErrorKind::UnknownAxis => "invalid axis name",
            ErrorKind::UnknownButton => "invalid button name",
            ErrorKind::InvalidParserState => "attempt to parse after unrecoverable error",
            ErrorKind::UnexpectedEnd => "mapping does not have all required fields",
        };

        f.write_fmt(format_args!("{} at {}", s, self.position))
    }
}

#[cfg(test)]
mod tests {
    use crate::mapping::parser::{ErrorKind, Parser};
    use crate::utils::PATH_SEPARATOR;

    #[test]
    fn test_all_sdl_mappings_for_parse_errors() {
        let included_mappings = include_str!(concat!(
            env!("OUT_DIR"),
            PATH_SEPARATOR!(),
            "gamecontrollerdb.txt"
        ))
        .lines();

        let mut errors = 0;
        let mut index = 0;
        for line in included_mappings {
            let mut parser = Parser::new(line);

            while let Some(token) = parser.next_token() {
                if let Err(ref e) = token {
                    if e.kind() != &ErrorKind::EmptyValue {
                        errors += 1;
                        println!("{:?}", e);
                        println!(
                            "{}: {} (...) {}\n",
                            index,
                            line.chars().take(50).collect::<String>(),
                            line.chars().skip(e.position).take(15).collect::<String>()
                        );

                        if e.kind() == &ErrorKind::InvalidParserState {
                            break;
                        }
                    }
                }
                index += 1;
            }
        }
        assert_eq!(errors, 0);
    }
}
