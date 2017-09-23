extern crate gilrs;
use gilrs::{Axis, Button, Event, EventType, Gilrs, Mapping};
use std::{io, u16};
use std::collections::HashMap;

fn main() {
    let mut gilrs = Gilrs::new();
    let mut mapping = Mapping::new();

    println!("Connected gamepads:");
    for (id, gp) in gilrs.gamepads() {
        println!(
            "{}: {} (mapping source: {:?})",
            id,
            gp.name(),
            gp.mapping_source()
        );
    }

    println!("Pleas select id:");
    let mut id = String::new();
    io::stdin()
        .read_line(&mut id)
        .expect("Failed to read from stdin");
    // Last char is '\n'
    let id = &id[..id.len() - 1];
    let id = id.parse().expect(&format!("{:?} is not valid id", id));

    // Discard unwanted events
    while let Some(_) = gilrs.next_event() {}

    println!(
        "Press east button on action pad (B on XBox gamepad layout). It will be used to \
         skip other mappings."
    );
    get_btn_nevc(&mut gilrs, id, u16::MAX).map(|nevc| mapping[Button::East] = nevc);
    let skip_btn = mapping[Button::East];

    println!("Press south button on action pad (A on XBox gamepad layout)");
    get_btn_nevc(&mut gilrs, id, skip_btn).map(|nevc| mapping[Button::South] = nevc);

    println!("Press west button on action pad (X on XBox gamepad layout)");
    get_btn_nevc(&mut gilrs, id, skip_btn).map(|nevc| mapping[Button::West] = nevc);

    println!("Press north button on action pad (Y on XBox gamepad layout)");
    get_btn_nevc(&mut gilrs, id, skip_btn).map(|nevc| mapping[Button::North] = nevc);

    println!("Press select button (back on XBox gamepad layout)");
    get_btn_nevc(&mut gilrs, id, skip_btn).map(|nevc| mapping[Button::Select] = nevc);

    println!("Press mode button (guide on XBox gamepad layout)");
    get_btn_nevc(&mut gilrs, id, skip_btn).map(|nevc| mapping[Button::Mode] = nevc);

    println!("Press start button");
    get_btn_nevc(&mut gilrs, id, skip_btn).map(|nevc| mapping[Button::Start] = nevc);

    println!("Press left stick");
    get_btn_nevc(&mut gilrs, id, skip_btn).map(|nevc| mapping[Button::LeftThumb] = nevc);

    println!("Press right stick");
    get_btn_nevc(&mut gilrs, id, skip_btn).map(|nevc| mapping[Button::RightThumb] = nevc);

    println!("Press first left trigger (LB on XBox gamepad layout)");
    get_axis_or_btn_nevc(&mut gilrs, id, skip_btn).map(|(el, nevc)| match el {
        ButtonOrAxis::Button => mapping[Button::LeftTrigger] = nevc,
        ButtonOrAxis::Axis => mapping[Axis::LeftTrigger] = nevc,
    });

    println!("Press second left trigger (LT on XBox gamepad layout)");
    get_axis_or_btn_nevc(&mut gilrs, id, skip_btn).map(|(el, nevc)| match el {
        ButtonOrAxis::Button => mapping[Button::LeftTrigger2] = nevc,
        ButtonOrAxis::Axis => mapping[Axis::LeftTrigger2] = nevc,
    });

    println!("Press first right trigger (RB on XBox gamepad layout)");
    get_axis_or_btn_nevc(&mut gilrs, id, skip_btn).map(|(el, nevc)| match el {
        ButtonOrAxis::Button => mapping[Button::RightTrigger] = nevc,
        ButtonOrAxis::Axis => mapping[Axis::RightTrigger] = nevc,
    });

    println!("Press second right trigger (RT on XBox gamepad layout)");
    get_axis_or_btn_nevc(&mut gilrs, id, skip_btn).map(|(el, nevc)| match el {
        ButtonOrAxis::Button => mapping[Button::RightTrigger2] = nevc,
        ButtonOrAxis::Axis => mapping[Axis::RightTrigger2] = nevc,
    });

    println!("Move left stick in X axis");
    get_axis_nevc(&mut gilrs, id, skip_btn).map(|nevc| mapping[Axis::LeftStickX] = nevc);

    println!("Move left stick in Y axis");
    get_axis_nevc(&mut gilrs, id, skip_btn).map(|nevc| mapping[Axis::LeftStickY] = nevc);

    println!("Move right stick in X axis");
    get_axis_nevc(&mut gilrs, id, skip_btn).map(|nevc| mapping[Axis::RightStickX] = nevc);

    println!("Move right stick in Y axis");
    get_axis_nevc(&mut gilrs, id, skip_btn).map(|nevc| mapping[Axis::RightStickY] = nevc);

    // Currently DPad mapping is not supported on any platform. If you know about gamepad with DPad
    // that generate ABS events different than ABS_HAT0X and ABS_HAT0Y (code 16 and 17) on Linux,
    // pleas create issue on https://gitlab.com/Arvamer/gilrs/issues

    let sdl_mapping = gilrs
        .gamepad_mut(id)
        .set_mapping(&mapping, None)
        .expect("Failed to set gamepad mapping");

    println!("\nSDL mapping:\n\n{}\n", sdl_mapping);
    println!("Gamepad mapped, you can test it now. Press CTRL-C to quit.\n");

    loop {
        while let Some(ev) = gilrs.next_event() {
            println!("{:?}", ev);
        }
    }
}

enum ButtonOrAxis {
    Button,
    Axis,
}

fn get_btn_nevc(g: &mut Gilrs, idx: usize, skip_btn: u16) -> Option<u16> {
    loop {
        while let Some(Event { id, event, .. }) = g.next_event() {
            if idx != id {
                continue;
            }
            match event {
                EventType::ButtonPressed(_, nevc) if nevc == skip_btn => return None,
                EventType::ButtonPressed(_, nevc) => return Some(nevc),
                _ => (),
            }
        }
    }
}

fn get_axis_nevc(g: &mut Gilrs, idx: usize, skip_btn: u16) -> Option<u16> {
    let mut state = HashMap::new();
    loop {
        while let Some(Event { id, event, .. }) = g.next_event() {
            if idx != id {
                continue;
            }
            match event {
                EventType::ButtonPressed(_, nevc) if nevc == skip_btn => return None,
                EventType::AxisChanged(_, val, nevc)
                    if val.abs() > 0.7 && state.get(&nevc).unwrap_or(&1.0f32).abs() <= 0.7 =>
                {
                    return Some(nevc)
                }
                EventType::AxisChanged(_, val, nevc) => {
                    state.insert(nevc, val);
                }
                _ => (),
            }
        }
    }
}

fn get_axis_or_btn_nevc(g: &mut Gilrs, idx: usize, skip_btn: u16) -> Option<(ButtonOrAxis, u16)> {
    let mut state = HashMap::new();
    loop {
        while let Some(Event { id, event, .. }) = g.next_event() {
            if idx != id {
                continue;
            }
            match event {
                EventType::ButtonPressed(_, nevc) if nevc == skip_btn => return None,
                EventType::ButtonPressed(_, nevc) => return Some((ButtonOrAxis::Button, nevc)),
                EventType::AxisChanged(_, val, nevc)
                    if val.abs() > 0.7 && state.get(&nevc).unwrap_or(&1.0f32).abs() <= 0.7 =>
                {
                    return Some((ButtonOrAxis::Axis, nevc))
                }
                EventType::AxisChanged(_, val, nevc) => {
                    state.insert(nevc, val);
                }
                _ => (),
            };
        }
    }
}
