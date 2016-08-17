extern crate gilrs;
extern crate env_logger;

use gilrs::Gilrs;
use gilrs::ff::EffectData;

fn main() {
    env_logger::init().unwrap();
    let mut gil = Gilrs::new();
    let mut effect = EffectData::default();
    effect.period = 1000;
    effect.magnitude = 20000;
    effect.replay.length = 5000;
    effect.envelope.attack_length = 1000;
    effect.envelope.fade_length = 1000;
    let effect_idx = gil.gamepad_mut(0).add_ff_effect(effect).unwrap();
    gil.gamepad_mut(0).ff_effect(effect_idx).unwrap().play(1);
    loop {
        for e in gil.poll_events() {
            println!("{:?}", e);
        }
    }
}
