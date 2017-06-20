use super::effect_source::{EffectSource, EffectState, Magnitude};
use super::time::{Ticks, TICK_DURATION};

use std::sync::mpsc::{self, Sender, Receiver};
use std::thread;
use std::time::{Duration, Instant};
use std::ops::Deref;

use platform::FfDevice;

use vec_map::VecMap;

#[derive(Debug)]
pub(crate) enum Message {
    Create { id: usize, effect: Box<EffectSource> },
    HandleCloned { id: usize },
    HandleDropped { id: usize },
    Play { id: usize },
    Open { id: usize, device: FfDevice },
    Close { id: usize },
    SetListenerPosition { id: usize, position: [f32; 3] }
}

#[derive(Debug)]
struct Device {
    inner: FfDevice,
    position: [f32; 3],
    gain: f32,
}

struct Effect {
    source: EffectSource,
    /// Number of created effect's handles.
    count: usize,
}

impl Effect {
    fn inc(&mut self) -> usize {
        self.count += 1;
        self.count
    }

    fn dec(&mut self) -> usize {
        self.count -= 1;
        self.count
    }
}

impl From<EffectSource> for Effect {
    fn from(source: EffectSource) -> Self {
        Effect {
            source,
            count: 1,
        }
    }
}

impl Deref for Effect {
    type Target = EffectSource;

    fn deref(&self) -> &Self::Target {
        &self.source
    }
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
    let mut effects = VecMap::<Effect>::new();
    let mut devices = VecMap::<Device>::new();
    let sleep_dur = Duration::from_millis(TICK_DURATION.into());
    let mut tick = Ticks(0);

    loop {
        let t1 = Instant::now();
        while let Ok(ev) = rx.try_recv() {
            match ev {
                Message::Create { id, effect } => {
                    effects.insert(id, (*effect).into());
                }
                Message::Play { id } => {
                    if let Some(effect) = effects.get_mut(id) {
                        effect.source.state = EffectState::Playing { since: tick }
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
                Message::SetListenerPosition { id, position } => {
                    if let Some(device) = devices.get_mut(id) {
                        device.position = position;
                    } else {
                        error!("{:?} with wrong ID", ev);
                    }
                }
                Message::HandleCloned { id } => {
                    if let Some(effect) = effects.get_mut(id) {
                        effect.inc();
                    } else {
                        error!("{:?} with wrong ID", ev);
                    }
                }
                Message::HandleDropped { id } => {
                    let mut drop = false;
                    if let Some(effect) = effects.get_mut(id) {
                        if effect.dec() == 0 {
                            drop = true;
                        }
                    } else {
                        error!("{:?} with wrong ID", ev);
                    }

                    if drop {
                        effects.remove(id);
                    }
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

fn combine_and_play(effects: &VecMap<Effect>, devices: &mut VecMap<Device>, tick: Ticks) {
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