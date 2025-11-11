//! Model of the tubing of a trumpet and associated types

use fixed::{
    traits::LossyFrom,
    types::{I1F15, U0F16, U12F4, U24F8, U4F4},
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
    blow: bool,
    embouchure_tightness: Embouchure,
    lung_pressure: BlowStrength,
}

impl TrumpetState {
    // Maps index (overtone) to embouchure at which the overtone resonates best
    // 1 => Low C
    // 2 => second line G
    // 3 => middle C
    // 4 => top space E
    // 5 => top of the staff G
    // 6 => Bb above the staff (31 cents sharp)
    // 7 => high C
    const EMBOUCHURE_TO_OVERTONE_MAP: [Embouchure; 9] = [
        Embouchure::unwrapped_from_str("0.000"),
        Embouchure::unwrapped_from_str("0.000"),
        Embouchure::unwrapped_from_str("0.060"),
        Embouchure::unwrapped_from_str("0.21"),
        Embouchure::unwrapped_from_str("0.3"),
        Embouchure::unwrapped_from_str("0.4"),
        Embouchure::unwrapped_from_str("0.5"),
        Embouchure::unwrapped_from_str("0.6"),
        Embouchure::unwrapped_from_str("0.999"),
    ];

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
        if !self.blow {
            return None;
        }

        for (i, &embouchure) in Self::EMBOUCHURE_TO_OVERTONE_MAP.iter().enumerate().rev() {
            if self.embouchure_tightness > embouchure {
                return Some(i as u8);
            }
        }

        None
    }

    const BENDABILITY_PER_OVERTONE: [U24F8; Self::EMBOUCHURE_TO_OVERTONE_MAP.len()] = [
        U24F8::unwrapped_from_str("2.0"),
        U24F8::unwrapped_from_str("1.5"),
        U24F8::unwrapped_from_str("1.5"),
        U24F8::unwrapped_from_str("0.5"),
        U24F8::unwrapped_from_str("0.5"),
        U24F8::unwrapped_from_str("0.4"),
        U24F8::unwrapped_from_str("0.3"),
        U24F8::unwrapped_from_str("0.1"),
        U24F8::unwrapped_from_str("0.0"),
    ];

    pub fn bend(&self) -> U24F8 {
        // The higher the lung pressure the more bend there is
        // the further from the 'ideal' embouchure for the overtone, the more bend there is
        let mut emb_bend = None;
        let mut bend_up = false;
        let mut overtone = 0;
        let mut closest_overtone = None;
        for embouchures in Self::EMBOUCHURE_TO_OVERTONE_MAP.windows(2) {
            let emb1 = embouchures[0];
            let emb2 = embouchures[1];

            if self.embouchure_tightness > emb1 && self.embouchure_tightness < emb2 {
                let max_diff = (emb2 - emb1) >> 1;
                if self.embouchure_tightness.abs_diff(emb1)
                    < self.embouchure_tightness.abs_diff(emb2)
                {
                    emb_bend = Some(max_diff - self.embouchure_tightness.abs_diff(emb1));
                    closest_overtone = Some(overtone);
                    bend_up = false;
                } else {
                    emb_bend = Some(max_diff - self.embouchure_tightness.abs_diff(emb2));
                    closest_overtone = Some(overtone + 1);
                    bend_up = true;
                }
            }
            overtone += 1;
        }

        if let (Some(emb_bend), Some(closest_overtone)) = (emb_bend, closest_overtone) {
            let bend = emb_bend;

            let bend_capacity = BlowStrength::unwrapped_from_str(".9") * self.lung_pressure;

            let bendability = Self::BENDABILITY_PER_OVERTONE[closest_overtone];

            if bend_up {
                U24F8::ONE
                    + (U24F8::lossy_from(bend) * U24F8::lossy_from(bend_capacity) * bendability)
            } else {
                U24F8::ONE
                    - (U24F8::lossy_from(bend) * U24F8::lossy_from(bend_capacity) * bendability)
            }
        } else {
            U24F8::ONE
        }
    }

    // TODO: move all the consts to a configuration

    pub fn volume(&self) -> U4F4 {
        U4F4::lossy_from(self.lung_pressure) + U4F4::unwrapped_from_str("0.2")
    }

    pub fn update(&mut self, event: TrumpetEvent) {
        self.valves.update(event);

        match event {
            TrumpetEvent::EmbouchureChange(fixed_i16) => self.embouchure_tightness = fixed_i16,
            TrumpetEvent::BlowStrengthChange(fixed_i16) => self.lung_pressure = fixed_i16,
            TrumpetEvent::BlowUp => self.blow = false,
            TrumpetEvent::BlowDown => self.blow = true,
            _ => (),
        }
    }
}

// https://www.yamaha.com/en/musical_instrument_guide/trumpet/mechanism/mechanism002.html
pub const BFLAT_TRUMPET: TrumpetDefinition = TrumpetDefinition {
    main_tube: U12F4::unwrapped_from_str("1470"),
    first_valve_tube: U12F4::unwrapped_from_str("190"),
    second_valve_tube: U12F4::unwrapped_from_str("95"),
    third_valve_tube: U12F4::unwrapped_from_str("285"),
    speed_of_sound: U24F8::unwrapped_from_str("343000"),
};

#[derive(Debug)]
pub struct Trumpet {
    def: TrumpetDefinition,
    pub state: TrumpetState,
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
        let fundamental = self.def.speed_of_sound / (U24F8::from(tube_length));

        Some(fundamental * U24F8::from_num(overtone + 1) * self.state.bend())
    }

    pub fn update(&mut self, events: &[TrumpetEvent]) -> Vec<Command, 4> {
        for &event in events {
            self.state.update(event);
        }

        let frequency = self.frequency();
        let volume = self.state.volume();

        let mut commands = Vec::new();
        // assume a change in state happened and the synth needs to be reconfigured
        if events.len() > 0 {
            let frequency = if let Some(f) = frequency {
                U12F4::wrapping_from_num(f)
            } else {
                U12F4::ZERO
            };

            commands
                .push(Command {
                    address: 0x0,
                    message: CommandMessage::Frequency(frequency, volume),
                })
                .expect("single push in length 4 vec is safe");
        }

        commands
    }
}
