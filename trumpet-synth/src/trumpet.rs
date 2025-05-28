//! Model of the tubing of a trumpet and associated types

use core::slice::SliceIndex;

use fixed::{
    traits::{Fixed, LossyFrom},
    types::{I12F4, I1F15, I24F8, U0F16, U12F4, U1F15, U24F8, U4F4},
};
use heapless::Vec;
use rytmos_synth::commands::{Command, CommandMessage};

use crate::interface::TrumpetEvent;

#[derive(Debug, Default, Clone, Copy)]
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

impl Into<bool> for ValveState {
    fn into(self) -> bool {
        match self {
            ValveState::Up => false,
            ValveState::Down => true,
        }
    }
}

#[derive(Debug, Default)]
pub struct Valves {
    pub first: ValveState,
    pub second: ValveState,
    pub third: ValveState,
}

impl Valves {
    pub fn set(&mut self, valve: Valve, state: ValveState) {
        match valve {
            Valve::First => self.first = state,
            Valve::Second => self.second = state,
            Valve::Third => self.third = state,
        }
    }

    pub fn update(&mut self, event: TrumpetEvent) {
        match event {
            TrumpetEvent::ValveUp(valve) => self.set(valve, ValveState::Up),
            TrumpetEvent::ValveDown(valve) => self.set(valve, ValveState::Down),
            _ => (),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Valve {
    First,
    Second,
    Third,
}

impl Into<usize> for Valve {
    fn into(self) -> usize {
        match self {
            Valve::First => 0,
            Valve::Second => 1,
            Valve::Third => 2,
        }
    }
}

impl From<usize> for Valve {
    fn from(value: usize) -> Self {
        match value {
            0 => Valve::First,
            1 => Valve::Second,
            2 => Valve::Third,
            // TODO: some sort of hardware panic handler that turns on a (red?) led somewhere in the rytmos library to make hardware panics easier to recognize
            _ => unreachable!(),
        }
    }
}

pub type Embouchure = U0F16;
pub type BlowStrength = U0F16;

/// All lengths in mm's
#[derive(Debug, Clone, Copy)]
pub struct TrumpetDefinition {
    main_tube: U12F4,
    first_valve_tube: U12F4,
    second_valve_tube: U12F4,
    third_valve_tube: U12F4,
    speed_of_sound: U24F8,
}

/// Represents the state of the mechanics of the trumpet, the "air" inside it,
/// and the vibrating lips, at some instant.
#[derive(Debug, Default)]
pub struct TrumpetState {
    valves: Valves,
    embouchure_tightness: Embouchure,
    lung_pressure: BlowStrength,
}

impl TrumpetState {
    pub fn tube_length(&self, def: &TrumpetDefinition) -> U12F4 {
        let mut length = def.main_tube;

        if self.valves.first.into() {
            length += def.first_valve_tube;
        }

        if self.valves.second.into() {
            length += def.second_valve_tube;
        }

        if self.valves.third.into() {
            length += def.third_valve_tube;
        }

        length
    }

    /// Based on the embouchure tightness and lung pressure, which overtone is playing?
    /// None if no note is playing. Frequency = fundamental * (overtone + 1)
    pub fn overtone(&self) -> Option<u8> {
        // 1 => Low C
        // 2 => second line G
        // 3 => middle C
        // 4 => top space E
        // 5 => top of the staff G
        // 6 => Bb above the staff (31 cents sharp)
        // 7 => high C
        // TODO: more overtones for the fun?
        // TODO: this function can use a lot of experimentation

        if self.lung_pressure > I1F15::from_num(0.1) {
            if self.embouchure_tightness > I1F15::from_num(0.8) {
                return Some(7);
            }

            if self.embouchure_tightness > I1F15::from_num(0.7) {
                return Some(6);
            }

            if self.embouchure_tightness > I1F15::from_num(0.6) {
                return Some(5);
            }

            if self.embouchure_tightness > I1F15::from_num(0.5) {
                return Some(4);
            }

            if self.embouchure_tightness > I1F15::from_num(0.4) {
                return Some(3);
            }

            if self.embouchure_tightness > I1F15::from_num(0.3) {
                return Some(2);
            }

            if self.embouchure_tightness > I1F15::from_num(0.2) {
                return Some(1);
            }
        }

        None
    }

    pub fn volume(&self) -> U4F4 {
        U4F4::lossy_from(self.lung_pressure)
    }

    pub fn update(&mut self, event: TrumpetEvent) {
        self.valves.update(event);

        match event {
            TrumpetEvent::EmbouchureChange(fixed_i16) => self.embouchure_tightness = fixed_i16,
            TrumpetEvent::BlowStrengthChange(fixed_i16) => self.lung_pressure = fixed_i16,
            _ => (),
        }
    }
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
    def: TrumpetDefinition,
    state: TrumpetState,
}

impl Trumpet {
    pub fn new(def: TrumpetDefinition) -> Self {
        Self {
            def,
            state: TrumpetState::default(),
        }
    }

    /// U12F4 goes from 0 to ~4095.94 in steps of 0.0625, high notes on a trumpet
    /// rarely exceed 2kHz so this accomodates frequencies nicely.
    pub fn frequency(&self) -> Option<U24F8> {
        let Some(overtone) = self.state.overtone() else {
            return None;
        };

        let tube_length = self.state.tube_length(&self.def);
        let fundamental = self.def.speed_of_sound / (U24F8::from_num(2) * U24F8::from(tube_length));

        Some(fundamental * U24F8::from_num(overtone + 1))
    }

    pub fn update(&mut self, events: &[TrumpetEvent]) -> Vec<Command, 4> {
        for &event in events {
            self.state.update(event);
        }

        let overtone = self.state.overtone();
        let tube_length = self.state.tube_length(&self.def);
        let fundamental = self.def.speed_of_sound / (U24F8::from_num(2) * U24F8::from(tube_length));

        let frequency = fundamental * U24F8::from_num(overtone.unwrap_or(0));
        let volume = self.state.volume();

        let mut commands = Vec::new();
        // assume a change in state happened and the synth needs to be reconfigured
        // TODO: optimize?
        if events.len() > 0 {
            // commands.push(CommandMessage::SetAttack(volume));
            commands
                .push(Command {
                    address: 0x0,
                    message: CommandMessage::Frequency(U12F4::wrapping_from_num(frequency), volume),
                })
                .expect("single push in length 4 vec is safe");
        }

        commands
    }
}
