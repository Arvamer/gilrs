extern crate gilrs;
extern crate env_logger;

use gilrs::Gilrs;
use gilrs::ff::{EffectBuilder, Envelope, Replay, BaseEffect, BaseEffectType};
use std::time::Duration;
use std::{thread, env};

fn main() {
    env_logger::init().unwrap();
    let mut gilrs = Gilrs::new();
    let support_ff = gilrs.gamepads().filter_map(|(id, gp)| if gp.is_ff_supported() { Some(id) } else { None }).collect::<Vec<_>>();

    let effect = EffectBuilder::new()
        .add_effect(BaseEffect {
            kind: BaseEffectType::Strong { magnitude: 60_000 },
            scheduling: Replay { play_for: 300 / 50, with_delay: 1000 / 50, ..Default::default() },
            envelope: Default::default(),
        })
        .add_effect(BaseEffect {
            kind: BaseEffectType::Weak { magnitude: 30_000 },
            ..Default::default()
        })
        .gamepads(&support_ff)
        .finish(&mut gilrs).unwrap();
    effect.play();

    thread::sleep(Duration::from_secs(11));
}
