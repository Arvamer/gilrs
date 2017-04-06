use std::ops::{Mul, AddAssign};
use std::u16;

use super::time::{Ticks, Repeat};
use super::base_effect::{BaseEffect, BaseEffectType};

use vec_map::VecMap;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum DistanceModel {
    Constant,
    Linear { ref_distance: f32 },
}

impl DistanceModel {
    fn attenuation(self, distance: f32) -> f32 {
        match self {
            DistanceModel::Linear { .. } => unimplemented!(),
            DistanceModel::Constant => 1.0,
        }
    }
}

impl Default for DistanceModel {
    fn default() -> Self {
        DistanceModel::Constant
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub(super) enum EffectState {
    Playing { since: Ticks },
    Stopped,
}

#[derive(Clone, PartialEq, Debug)]
pub(crate) struct EffectSource {
    base_effects: Vec<BaseEffect>,
    pub(super) devices: VecMap<()>,
    repeat: Repeat,
    dist_model: DistanceModel,
    position: [f32; 3],
    gain: f32,
    pub(super) state: EffectState,
}

impl EffectSource {
    pub (super) fn new(base_effects: Vec<BaseEffect>,
                       devices: VecMap<()>,
                       repeat: Repeat,
                       dist_model: DistanceModel,
                       position: [f32; 3],
                       gain: f32)
                       -> Self
    {
        EffectSource {
            base_effects,
            devices,
            repeat,
            dist_model,
            position,
            gain,
            state: EffectState::Stopped,
        }
    }

    pub(super) fn combine_base_effects(&self, ticks: Ticks, actor_pos: [f32; 3]) -> Magnitude {
        let ticks = match self.state {
            EffectState::Playing { since } =>{
                debug_assert!(ticks >= since);
                ticks - since
            },
            EffectState::Stopped => return Magnitude::zero(),
        };

        match self.repeat {
            Repeat::For(max_dur) if max_dur > ticks => {
                // TODO: Maybe change to new state, "Ended"?
                // self.state = EffectState::Stopped;
                return Magnitude::zero();
            }
            _ => ()
        }

        let attenuation = self.dist_model.attenuation(self.position.distance(actor_pos)) * self.gain;
        if attenuation < 0.05 {
            return Magnitude::zero()
        }

        let mut final_magnitude = Magnitude::zero();
        for effect in &self.base_effects {
            match effect.magnitude_at(ticks) {
                BaseEffectType::Strong { magnitude } => final_magnitude.strong = final_magnitude.strong.saturating_add(magnitude),
                BaseEffectType::Weak { magnitude } => final_magnitude.weak = final_magnitude.weak.saturating_add(magnitude),
                BaseEffectType::__Nonexhaustive => (),
            };
        }
        final_magnitude * attenuation
    }
}

/// (strong, weak) pair.
#[derive(Copy, Clone, Debug)]
pub(super) struct Magnitude {
    pub strong: u16,
    pub weak: u16,
}

impl Magnitude {
    pub fn zero() -> Self {
        Magnitude { strong: 0, weak: 0 }
    }
}

impl Mul<f32> for Magnitude {
    type Output = Magnitude;

    fn mul(self, rhs: f32) -> Self::Output {
        debug_assert!(rhs >= 0.0);
        let strong = self.strong as f32 * rhs;
        let strong = if strong > u16::MAX as f32 { u16::MAX } else { strong as u16 };
        let weak = self.weak as f32 * rhs;
        let weak = if weak > u16::MAX as f32 { u16::MAX } else { weak as u16 };
        Magnitude { strong: strong, weak: weak }
    }
}

impl AddAssign for Magnitude {
    fn add_assign(&mut self, rhs: Magnitude) {
        self.strong = self.strong.saturating_add(rhs.strong);
        self.weak = self.weak.saturating_add(rhs.weak);
    }
}

trait SliceVecExt {
    type Base;

    fn distance(self, from: Self) -> Self::Base;
}

impl  SliceVecExt for [f32; 3] {
    type Base = f32;

    fn distance(self, from: Self) -> f32 {
        ((from[0] - self[0]).powi(2) + (from[1] - self[1]).powi(2) + (from[2] - self[2]).powi(2)).sqrt()
    }
}
