extern crate gilrs;
extern crate env_logger;

use gilrs::Gilrs;
use gilrs::ff::{EffectBuilder, Replay, BaseEffect, BaseEffectType, Ticks};
use std::time::Duration;
use std::{thread};

fn main() {
    env_logger::init().unwrap();
    let mut gilrs = Gilrs::new();
    let support_ff = gilrs.gamepads().filter_map(|(id, gp)| if gp.is_ff_supported() { Some(id) } else { None }).collect::<Vec<_>>();

    let duration = Ticks::from_ms(150);
    let effect = EffectBuilder::new()
        .add_effect(BaseEffect {
            kind: BaseEffectType::Strong { magnitude: 60_000 },
            scheduling: Replay { play_for: duration, with_delay: duration * 3, ..Default::default() },
            envelope: Default::default(),
        })
        .add_effect(BaseEffect {
            kind: BaseEffectType::Weak { magnitude: 60_000 },
            scheduling: Replay { after: duration * 2, play_for: duration, with_delay: duration * 3 },
            ..Default::default()
        })
        .gamepads(&support_ff)
        .finish(&mut gilrs).unwrap();
    effect.play().unwrap();

    thread::sleep(Duration::from_secs(11));
}
