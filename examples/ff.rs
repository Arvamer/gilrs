extern crate gilrs;
extern crate env_logger;

use gilrs::Gilrs;
use gilrs::ff::EffectData;
use std::time::Duration;
use std::thread;

fn main() {
    env_logger::init().unwrap();
    let mut gil = Gilrs::new();
    let mut effect = EffectData::default();
    effect.period = 1000;
    effect.magnitude = 20000;
    effect.replay.length = 5000;
    effect.envelope.attack_length = 1000;
    effect.envelope.fade_length = 1000;

    let effect_idx = gil[0].add_ff_effect(effect).unwrap();
    let _ = gil[0].ff_effect(effect_idx).unwrap().play(1);

    thread::sleep(Duration::from_secs(5));
}
