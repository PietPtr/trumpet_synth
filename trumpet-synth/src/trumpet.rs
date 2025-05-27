//! Types and utilities for inputs of the device

use fixed::types::I1F15;

pub enum ValveState {
    Up,
    Down,
}

impl From<bool> for ValveState {
    fn from(value: bool) -> Self {
        match value {
            true => ValveState::Down,
            false => ValveState::Up,
        }
    }
}

pub struct Valves {
    pub first: ValveState,
    pub second: ValveState,
    pub third: ValveState,
}

pub type Embouchure = I1F15;
pub type BlowStrength = I1F15;
