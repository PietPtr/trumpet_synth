use common::debouncer::Debouncer;
use heapless::Vec;

use crate::{
    io::{Fifo, Inputs, TrumpetInputState, IO},
    trumpet::{BlowStrength, Embouchure, Valve},
};

// TODO: different place for some of these structs/impls?

/// Abstraction over raw input readings, takes care of e.g. debouncing and
/// rescaling the potentiometer values.
pub struct TrumpetInputs<INPUTS> {
    inputs: INPUTS,
    valve_debouncers: [Debouncer; 3],
    events: Vec<TrumpetEvent, 4>,
    last_trumpet_state: TrumpetInputState,
}

const DEBOUNCE_TIME: u32 = 10;

pub enum TrumpetEvent {
    ValveUp(Valve),
    ValveDown(Valve),
    EmbouchureChange(Embouchure),
    BlowStrengthChange(BlowStrength),
}

// TODO: like Clavier in polypicophonic, turns embedded IO to a more or less event based thing
impl<INPUTS: Inputs> TrumpetInputs<INPUTS> {
    pub fn new(inputs: INPUTS) -> Self {
        Self {
            inputs,
            valve_debouncers: [Debouncer::new(DEBOUNCE_TIME); 3],
            events: Vec::new(),
            last_trumpet_state: TrumpetInputState::default(),
        }
    }

    fn update_debouncers(&mut self, state: TrumpetInputState) {
        for (id, debouncer) in self.valve_debouncers.iter_mut().enumerate() {
            debouncer.update(state.valve(Valve::from(id)));
        }
    }

    pub fn events(&self) -> &[TrumpetEvent] {
        &self.events
    }

    fn debouncer_is_high(&self, valve: Valve) -> bool {
        let valve = valve as usize;
        let Some(debouncer) = self.valve_debouncers.get(valve) else {
            panic!("No debouncer found for KeyID {:?}.", valve);
        };

        debouncer.is_high().unwrap_or(false)
    }

    pub fn update_events(&mut self) {
        let mut current_state = TrumpetInputState::read_from(&mut self.inputs);
        self.update_debouncers(current_state);

        current_state.first = self.debouncer_is_high(Valve::First);
        current_state.second = self.debouncer_is_high(Valve::Second);
        current_state.third = self.debouncer_is_high(Valve::Third);

        TODO: make logic for detecting valve changes / changes in pot values

        self.last_trumpet_state = current_state;
    }
}

// TODO: tests are possible using this library setup, e.g. fuzzers for checking panic-free-ness

pub struct TrumpetInterface<FIFO, INPUTS> {
    fifo: FIFO,
    inputs: TrumpetInputs<INPUTS>,
}

impl<FIFO: Fifo, INPUTS: Inputs> TrumpetInterface<FIFO, INPUTS> {
    pub fn new(io: IO<FIFO, INPUTS>) -> Self {
        Self {
            fifo: io.fifo,
            inputs: TrumpetInputs::new(io.inputs),
        }
    }

    pub fn run(&mut self) {
        self.inputs.update_events();
    }
}
