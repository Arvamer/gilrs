extern crate gilrs;
extern crate env_logger;

use gilrs::{Gilrs, Event, Button, Axis};
use gilrs::ff::{EffectBuilder, Envelope, Replay, BaseEffect, BaseEffectType, Ticks, DistanceModel};

use std::time::Duration;
use std::ops::IndexMut;
use std::{thread, env};
use std::io::{self, Write};

fn main() {
    env_logger::init().unwrap();
    let mut gilrs = Gilrs::new();

    println!("Connected gamepads:");

    let mut support_ff = Vec::new();
    for (idx, gp) in gilrs.gamepads() {
        let ff = gp.is_ff_supported();
        println!("{}) {} ({})", idx, gp.name(),
                 if ff { "Force feedback supported" } else { "Force feedback not supported" });
        if ff {
            support_ff.push(idx);
        }
    }

    println!("----------------------------------------");
    println!("Use sticks to move listener. Press east button on action pad to quit.");

    let pos1 = [-100.0, 0.0, 0.0];
    let pos2 = [100.0, 50.0, 0.0];

    let effect_builder = EffectBuilder::new()
        .add_effect(BaseEffect {
            kind: BaseEffectType::Strong { magnitude: 45_000 },
            ..Default::default()
        })
        .add_effect(BaseEffect {
            kind: BaseEffectType::Weak { magnitude: 45_000 },
            ..Default::default()
        })
        .dist_model(DistanceModel::Inverse { ref_distance: 10.0, rolloff_factor: 0.5 })
        .gamepads(&support_ff)
        .clone();

    let left_effect = effect_builder.clone()
        .position(pos1)
        .finish(&mut gilrs).unwrap();
    let right_effect = effect_builder.clone()
        .position(pos2)
        .finish(&mut gilrs).unwrap();


    left_effect.play();
    right_effect.play();

    println!("Playing two effects…");
    println!("Position of effect sources: {:?}, {:?}", pos1, pos2);

    let mut listeners = support_ff.iter()
        .map(|&idx| (idx, [0.0, 0.0, 0.0]))
        .collect::<Vec<_>>();

    'main: loop {
        for (_, ev) in gilrs.poll_events() {
            match ev {
                Event::ButtonReleased(Button::East, ..) => break 'main,
                _ => (),
            }
        }

        for &mut (idx, ref mut pos) in &mut listeners {
            let velocity = 0.5;

            let gp = gilrs.gamepad(idx);
            let (sx, sy) = (gp.value(Axis::LeftStickX), gp.value(Axis::LeftStickY));

            if sx.abs() > 0.5 || sy.abs() > 0.5 {
                if sx.abs() > 0.5 {
                    pos[0] += velocity * sx.signum();
                }
                if sy.abs() > 0.5 {
                    pos[1] += velocity * sy.signum();
                }

                gilrs.set_listener_position(idx, *pos).unwrap();
                print!("\rPosition of listener {:2} has changed: [{:6.1}, {:6.1}]",
                       idx, pos[0], pos[1]);
                io::stdout().flush().unwrap();
            }
        }

        thread::sleep(Duration::from_millis(16));
    }
}
