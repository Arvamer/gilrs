
use super::State;
use gamepad::{Event, EventType};

use std::time::{Duration, SystemTime};

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Noise {
    pub threshold: f32,
}

impl Noise {
    pub fn new() -> Self {
        Noise { threshold: 0.01 }
    }
}

impl FilterFn for Noise {
    fn filter(&self, ev: Option<Event>, state: &State) -> Option<Event> {
        match ev {
            Some(Event {
                     event: EventType::AxisChanged(_, val, axis),
                     id,
                     ..
                 }) => {
                match state.axis_data_nec(id, axis) {
                    Some(data) if (val - data.value()).abs() < self.threshold => None,
                    _ => ev,
                }
            }
            _ => ev,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Repeat {
    pub after: Duration,
    pub every: Duration,
}

impl Repeat {
    pub fn new() -> Self {
        Repeat {
            after: Duration::from_millis(500),
            every: Duration::from_millis(30),
        }
    }
}

impl FilterFn for Repeat {
    fn filter(&self, ev: Option<Event>, state: &State) -> Option<Event> {
        match ev {
            Some(ev) => Some(ev),
            None => {
                let now = SystemTime::now();
                for (id, gamepad_state) in state.gamepads() {
                    for (nec, btn_data) in gamepad_state.buttons() {
                        let nec = nec as u16;
                        match (
                            btn_data.is_pressed(),
                            btn_data.is_repeating(),
                            now.duration_since(btn_data.timestamp()),
                        ) {
                            (true, false, Ok(dur)) if dur >= self.after => {
                                return Some(Event {
                                    id,
                                    event: EventType::ButtonRepeated(
                                        gamepad_state.nec_to_btn(nec),
                                        nec,
                                    ),
                                    time: btn_data.timestamp() + self.after,
                                })
                            }
                            (true, true, Ok(dur)) if dur >= self.every => {
                                return Some(Event {
                                    id,
                                    event: EventType::ButtonRepeated(
                                        gamepad_state.nec_to_btn(nec),
                                        nec,
                                    ),
                                    time: btn_data.timestamp() + self.every,
                                })
                            }
                            _ => (),
                        }
                    }
                }
                None
            }
        }
    }
}

pub trait Filter {
    fn filter<F: FilterFn>(&mut self, filter: &F, state: &State) -> Option<Event>;
}

pub trait FilterFn {
    fn filter(&self, ev: Option<Event>, state: &State) -> Option<Event>;
}

impl Filter for Option<Event> {
    fn filter<F: FilterFn>(&mut self, filter: &F, state: &State) -> Option<Event> {
        filter.filter(*self, state)
    }
}

impl Filter for Event {
    fn filter<F: FilterFn>(&mut self, filter: &F, state: &State) -> Option<Event> {
        filter.filter(Some(*self), state)
    }
}
