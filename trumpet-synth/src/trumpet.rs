//! Model of the tubing of a trumpet and associated types

use fixed::types::{I12F4, I1F15, I24F8, U12F4, U24F8};

#[derive(Debug, Default)]
pub enum ValveState {
    #[default]
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

#[derive(Debug, Default)]
pub struct Valves {
    pub first: ValveState,
    pub second: ValveState,
    pub third: ValveState,
}

pub type Embouchure = I1F15;
pub type BlowStrength = I1F15;

/// All lengths in mm's
#[derive(Debug)]
pub struct TrumpetDefinition {
    main_tube: U12F4,
    first_valve_tube: U12F4,
    second_valve_tube: U12F4,
    third_valve_tube: U12F4,
    speed_of_sound: U24F8,
}

// https://www.yamaha.com/en/musical_instrument_guide/trumpet/mechanism/mechanism002.html
pub const BFLAT_TRUMPET: TrumpetDefinition = TrumpetDefinition {
    main_tube: U12F4::unwrapped_from_str("1480"),
    first_valve_tube: U12F4::unwrapped_from_str("160"),
    second_valve_tube: U12F4::unwrapped_from_str("70"),
    third_valve_tube: U12F4::unwrapped_from_str("270"),
    speed_of_sound: U24F8::unwrapped_from_str("34300"),
};

#[derive(Debug)]
pub struct Trumpet {
    valves: Valves,
    def: TrumpetDefinition,
}

impl Trumpet {
    pub fn new(def: TrumpetDefinition) -> Self {
        Self {
            valves: Valves::default(),
            def,
        }
    }

    /// U12F4 goes from 0 to ~4095.94 in steps of 0.0625, high notes on a trumpet
    /// rarely exceed 2kHz so this accomodates frequencies nicely.
    pub fn frequency(emb: I1F15, blow: I1F15) -> U12F4 {
        todo!()
    }
}
