#![allow(unused_variables)]

use super::gamepad::Gamepad;
use ff::EffectData;

#[derive(Debug)]
pub struct Effect {}

impl Effect {
    pub fn new(gamepad: &Gamepad, data: EffectData) -> Option<Self> {
        None
    }

    pub fn upload(&mut self, data: EffectData) -> Option<()> {
        None
    }

    pub fn play(&mut self, n: u16) {}

    pub fn stop(&mut self) {}
}
