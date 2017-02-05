extern crate gilrs;
extern crate env_logger;

use gilrs::Gilrs;
use gilrs::ff::{EffectData, EffectType, Waveform, Envelope, Replay};
use std::time::Duration;
use std::{thread, env};

fn main() {
    env_logger::init().unwrap();
    let mut gilrs = Gilrs::new();
    let effect =
        if env::args().nth(1).and_then(|s| if s == "rumble" { Some(()) } else { None }).is_none() {
            EffectData {
                kind: EffectType::Periodic {
                    wave: Waveform::Sine,
                    period: 1000,
                    magnitude: 30_000,
                    offset: 0,
                    phase: 0,
                    envelope: Envelope {
                        attack_length: 2000,
                        attack_level: 0,
                        fade_length: 1000,
                        fade_level: 5000,
                    },
                },
                replay: Replay {
                    length: 5000,
                    delay: 0,
                },
                ..Default::default()
            }
        } else {
            EffectData {
                replay: Replay {
                    length: 5000,
                    delay: 0,
                },
                kind: EffectType::Rumble {
                    strong: 20_000,
                    weak: 0,
                },
                ..Default::default()
            }
        };

    let effect_idx = gilrs[0].add_ff_effect(effect).unwrap();
    let _ = gilrs[0].ff_effect(effect_idx).unwrap().play(2);

    thread::sleep(Duration::from_secs(11));
}
