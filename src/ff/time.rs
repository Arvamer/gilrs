use std::ops::{Add, AddAssign, Sub, SubAssign, Rem};

use utils;

pub(crate) const TICK_DURATION: u32 = 50;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Ticks(pub(super) u32);

impl Ticks {
    pub fn from_ms(dur: u32) -> Self {
        Ticks(utils::ceil_div(dur, TICK_DURATION))
    }

    pub(super) fn inc(&mut self) {
        self.0 += 1
    }

    pub(super) fn checked_sub(self, rhs: Ticks) -> Option<Ticks> {
        self.0.checked_sub(rhs.0).map(|t| Ticks(t))
    }
}

impl Add for Ticks {
    type Output = Ticks;

    fn add(self, rhs: Ticks) -> Self::Output {
        Ticks(self.0 + rhs.0)
    }
}

impl AddAssign for Ticks {
    fn add_assign(&mut self, rhs: Ticks) {
        self.0 += rhs.0
    }
}

impl Sub for Ticks {
    type Output = Ticks;

    fn sub(self, rhs: Ticks) -> Self::Output {
        Ticks(self.0 - rhs.0)
    }
}

impl SubAssign for Ticks {
    fn sub_assign(&mut self, rhs: Ticks) {
        self.0 -= rhs.0
    }
}

impl Rem for Ticks {
    type Output = Ticks;

    fn rem(self, rhs: Ticks) -> Self::Output {
        Ticks(self.0 % rhs.0)
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Repeat {
    Infinitely,
    For(Ticks),
}

impl Default for Repeat {
    fn default() -> Self {
        Repeat::Infinitely
    }
}
