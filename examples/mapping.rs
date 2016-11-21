extern crate gilrs;
use gilrs::{Gilrs, Mappings, Button, Axis, Event};
use std::io::{self, BufRead};
use std::collections::HashMap;

fn main() {
    let mut gilrs = Gilrs::new();
    let mut mapping = Mappings::new();

    println!("Connected gamepads:");
    for (id, gp) in gilrs.gamepads() {
        println!("{}: {}", id, gp.name());
    }

    println!("Pleas select id:");
    let mut id = String::new();
    io::stdin().read_line(&mut id).expect("Failed to read from stdio");
    let id = &id[..id.len() - 1];
    let id = id.parse().expect(&format!("{:?} is not valid id", id));

    // Discard unwanted events
    for _ in gilrs.poll_events() {}

    println!("Press south button on action pad (A on XBox gamepad layout)");
    get_btn_nevc(&mut gilrs, id).map(|nevc| mapping[Button::South] = nevc);

    println!("Press east button on action pad (B on XBox gamepad layout)");
    get_btn_nevc(&mut gilrs, id).map(|nevc| mapping[Button::East] = nevc);

    println!("Press north button on action pad (Y on XBox gamepad layout)");
    get_btn_nevc(&mut gilrs, id).map(|nevc| mapping[Button::North] = nevc);

    println!("Press west button on action pad (X on XBox gamepad layout)");
    get_btn_nevc(&mut gilrs, id).map(|nevc| mapping[Button::West] = nevc);

    println!("Press select button (back on XBox gamepad layout)");
    get_btn_nevc(&mut gilrs, id).map(|nevc| mapping[Button::Select] = nevc);

    println!("Press mode button (guide on XBox gamepad layout)");
    get_btn_nevc(&mut gilrs, id).map(|nevc| mapping[Button::Mode] = nevc);

    println!("Press start button");
    get_btn_nevc(&mut gilrs, id).map(|nevc| mapping[Button::Start] = nevc);

    println!("Press left stick");
    get_btn_nevc(&mut gilrs, id).map(|nevc| mapping[Button::LeftThumb] = nevc);

    println!("Press right stick");
    get_btn_nevc(&mut gilrs, id).map(|nevc| mapping[Button::RightThumb] = nevc);

    println!("Press first left trigger (LB on XBox gamepad layout)");
    get_axis_or_btn_nevc(&mut gilrs, id).map(|(el, nevc)| {
        match el {
            ButtonOrAxis::Button => mapping[Button::LeftTrigger] = nevc,
            ButtonOrAxis::Axis => mapping[Axis::LeftTrigger] = nevc,
        }
    });

    println!("Press second left trigger (LT on XBox gamepad layout)");
    get_axis_or_btn_nevc(&mut gilrs, id).map(|(el, nevc)| {
        match el {
            ButtonOrAxis::Button => mapping[Button::LeftTrigger2] = nevc,
            ButtonOrAxis::Axis => mapping[Axis::LeftTrigger2] = nevc,
        }
    });

    println!("Press first right trigger (RB on XBox gamepad layout)");
    get_axis_or_btn_nevc(&mut gilrs, id).map(|(el, nevc)| {
        match el {
            ButtonOrAxis::Button => mapping[Button::RightTrigger] = nevc,
            ButtonOrAxis::Axis => mapping[Axis::RightTrigger] = nevc,
        }
    });

    println!("Press second right trigger (RT on XBox gamepad layout)");
    get_axis_or_btn_nevc(&mut gilrs, id).map(|(el, nevc)| {
        match el {
            ButtonOrAxis::Button => mapping[Button::RightTrigger2] = nevc,
            ButtonOrAxis::Axis => mapping[Axis::RightTrigger2] = nevc,
        }
    });

    println!("Move left stick in X axis");
    get_axis_nevc(&mut gilrs, id).map(|nevc| mapping[Axis::LeftStickX] = nevc);

    println!("Move left stick in Y axis");
    get_axis_nevc(&mut gilrs, id).map(|nevc| mapping[Axis::LeftStickY] = nevc);

    println!("Move right stick in X axis");
    get_axis_nevc(&mut gilrs, id).map(|nevc| mapping[Axis::RightStickX] = nevc);

    println!("Move right stick in Y axis");
    get_axis_nevc(&mut gilrs, id).map(|nevc| mapping[Axis::RightStickY] = nevc);

    gilrs.gamepad_mut(id).set_mappings(&mapping, None).expect("Failed to set gamepad mappings");
    println!("Gamepad mapped, you can test it now");

    loop {
        for ev in gilrs.poll_events() {
            println!("{:?}", ev);
        }
    }

}

enum ButtonOrAxis {
    Button,
    Axis,
}

fn get_btn_nevc(g: &mut Gilrs, id: usize) -> Option<u16> {
    loop {
        for (i, ev) in g.poll_events() {
            if id != i { continue }
            match ev {
                Event::ButtonPressed(_, nevc) => return Some(nevc),
                _ => (),
            }
        }
    }
}

fn get_axis_nevc(g: &mut Gilrs, id: usize) -> Option<u16> {
    let mut state = HashMap::new();
    loop {
        for (i, ev) in g.poll_events() {
            if id != i { continue }
            match ev {
                Event::AxisChanged(_, val, nevc)
                    if val.abs() > 0.7 && state.get(&nevc).unwrap_or(&1.0f32).abs() <= 0.7
                    => return Some(nevc),
                Event::AxisChanged(_, val, nevc)
                    => { state.insert(nevc, val); },
                _ => (),
            }
        }
    }
}

fn get_axis_or_btn_nevc(g: &mut Gilrs, id: usize) -> Option<(ButtonOrAxis, u16)> {
    let mut state = HashMap::new();
    loop {
        for (i, ev) in g.poll_events() {
            if id != i { continue }
            match ev {
                Event::ButtonPressed(_, nevc) => return Some((ButtonOrAxis::Button, nevc)),
                Event::AxisChanged(_, val, nevc)
                    if val.abs() > 0.7 && state.get(&nevc).unwrap_or(&1.0f32).abs() <= 0.7
                    => return Some((ButtonOrAxis::Axis, nevc)),
                Event::AxisChanged(_, val, nevc)
                    => { state.insert(nevc, val); },
                _ => (),
            };
        }
    }
}
