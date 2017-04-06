use super::effect_source::{EffectSource, EffectState, Magnitude};
use super::time::{Ticks, TICK_DURATION};

use std::sync::mpsc::{self, Sender, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use platform::FfDevice;

use vec_map::VecMap;

#[derive(Debug)]
pub(crate) enum Message {
    Update { id: usize, effect: EffectSource },
    Play { id: usize },
    Open { id: usize, device: FfDevice },
    Close { id: usize },
}

#[derive(Debug)]
struct Device {
    inner: FfDevice,
    position: [f32; 3],
    gain: f32,
}


impl From<FfDevice> for Device {
    fn from(inner: FfDevice) -> Self {
        Device {
            inner: inner,
            position: [0.0, 0.0, 0.0],
            gain: 1.0,
        }
    }
}

pub(crate) fn run(rx: Receiver<Message>) {
    let mut effects = VecMap::new();
    let mut devices = VecMap::new();
    let sleep_dur = Duration::from_millis(TICK_DURATION.into());
    let mut tick = Ticks(0);

    loop {
        let t1 = Instant::now();
        while let Ok(ev) = rx.try_recv() {
            match ev {
                Message::Update { id, effect } => {
                    effects.insert(id, effect);
                }
                Message::Play { id } => {
                    if let Some(effect) = effects.get_mut(id) {
                        effect.state = EffectState::Playing { since: tick }
                    } else {
                        error!("{:?} with wrong ID", ev);
                    }
                }
                Message::Open {id, device } => {
                    devices.insert(id, device.into());
                },
                Message::Close { id } => {
                    devices.remove(id);
                }
            }
        }

        combine_and_play(&effects, &mut devices, tick);

        let dur = Instant::now().duration_since(t1);
        if dur > sleep_dur {
            // TODO: Should we add dur - sleep_dur to next iteration's dur?
            warn!("One iteration of a force feedback loop took more than {}ms!", TICK_DURATION);
        } else {
            thread::sleep(sleep_dur - dur);
        }
        tick.inc();
    }
}

pub(crate) fn init() -> Sender<Message> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || run(rx));
    tx
}

fn combine_and_play(effects: &VecMap<EffectSource>, devices: &mut VecMap<Device>, tick: Ticks) {
    for (dev_id, dev) in devices {
        let mut magnitude = Magnitude::zero();
        for (_, effect) in effects {
            if effect.devices.contains_key(dev_id) {
                magnitude += effect.combine_base_effects(tick, dev.position);
            }
        }
        dev.inner.set_ff_state(magnitude.strong, magnitude.weak);
    }
}