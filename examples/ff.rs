extern crate gilrs;
use gilrs::Gilrs;
use gilrs::ff::{Effect, EffectData};

fn main() {
    let mut gil = Gilrs::new();
    let mut effect = EffectData::default();
    effect.period = 1000;
    effect.magnitude = 20000;
    effect.replay.length = 5000;
    effect.envelope.attack_length = 1000;
    effect.envelope.fade_length = 1000;
    let mut effect = Effect::new(gil.gamepad(0), effect).unwrap();
    effect.play(1);
    loop {
    }
}
