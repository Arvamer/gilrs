use std::ops::Mul;

use super::time::Ticks;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum BaseEffectType {
    Weak { magnitude: u16 },
    Strong { magnitude: u16 },
    #[doc(hidden)]
    __Nonexhaustive,
}

impl BaseEffectType {
    fn magnitude(&self) -> u16 {
        match *self {
            BaseEffectType::Weak { magnitude } => magnitude,
            BaseEffectType::Strong { magnitude } => magnitude,
            BaseEffectType::__Nonexhaustive => unreachable!(),
        }
    }
}

impl Mul<f32> for BaseEffectType {
    type Output = BaseEffectType;

    fn mul(self, rhs: f32) -> Self::Output {
        let mg = (self.magnitude() as f32 * rhs) as u16;
        match self {
            BaseEffectType::Weak { .. } => BaseEffectType::Weak { magnitude: mg },
            BaseEffectType::Strong { .. } => BaseEffectType::Strong { magnitude: mg },
            BaseEffectType::__Nonexhaustive => unreachable!(),
        }
    }
}

impl Default for BaseEffectType {
    fn default() -> Self {
        BaseEffectType::Weak { magnitude: 0 }
    }
}

#[derive(Copy, Clone, PartialEq, Debug, Default)]
pub struct BaseEffect {
    pub kind: BaseEffectType,
    pub scheduling: Replay,
    // TODO: maybe allow other f(t)?
    pub envelope: Envelope,
}

impl BaseEffect {
    /// Returns `Weak` or `Strong` after applying envelope.
    pub(super) fn magnitude_at(&self, ticks: Ticks) -> BaseEffectType {
        if let Some(wrapped) = self.scheduling.wrap(ticks) {
            let att = self.scheduling.at(wrapped) * self.envelope.at(wrapped, self.scheduling.play_for);
            self.kind * att
        } else {
            self.kind * 0.0
        }
    }
}

// TODO: Image with "envelope"
#[derive(Copy, Clone, PartialEq, Debug, Default)]
/// Envelope shaped gain(time) function.
pub struct Envelope {
    pub attack_length: Ticks,
    pub attack_level: f32,
    pub fade_length: Ticks,
    pub fade_level: f32,
}

impl Envelope {
    fn at(&self, ticks: Ticks, dur: Ticks) -> f32 {
        debug_assert!(self.fade_length < dur);
        debug_assert!(self.attack_length + self.fade_length < dur);

        if ticks < self.attack_length {
            self.attack_level + ticks.0 as f32 * (1.0 - self.attack_level) / self.attack_length.0 as f32
        } else if ticks + self.fade_length > dur {
            1.0 + (ticks + self.fade_length - dur).0 as f32 * (self.fade_level - 1.0) / self.fade_length.0 as f32
        } else {
            1.0
        }
    }
}

/// Defines scheduling of the force feedback effect
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Replay {
    pub after: Ticks,
    pub play_for: Ticks,
    pub with_delay: Ticks,
}

impl Replay {
    fn at(&self, ticks: Ticks) -> f32 {
        match ticks.checked_sub(self.after) {
            Some(ticks) => {
                if ticks.0 >= self.play_for.0 {
                    0.0
                } else {
                    1.0
                }
            }
            None => 0.0,
        }
    }

    /// Returns duration of effect calculated as `play_for + with_delay`.
    pub fn dur(&self) -> Ticks {
        self.play_for + self.with_delay
    }

    /// Returns `None` if effect hasn't started or wrapped value
    fn wrap(&self, ticks: Ticks) -> Option<Ticks> {
        ticks.checked_sub(self.after).map(|t| t % self.dur())
    }
}

impl Default for Replay {
    fn default() -> Self {
        Replay {
            after: Ticks(0),
            play_for: Ticks(1),
            with_delay: Ticks(0),
        }
    }
}
