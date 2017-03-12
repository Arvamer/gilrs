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

    for (_, gp) in gil.gamepads_mut().filter(|&(_, ref gp)| gp.is_ff_supported()) {
        // In real game don't do this — play ff effect only on gamepads which are used.
        let effect_idx = gp.add_ff_effect(effect).unwrap();
        let _ = gp.ff_effect(effect_idx).unwrap().play(1);
    }

    thread::sleep(Duration::from_secs(5));
}
