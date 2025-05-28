use common::debouncer::Debouncer;
use fixed::types::I1F15;
use heapless::Vec;

use crate::{
    io::{Fifo, Inputs, TrumpetInputState, IO},
    trumpet::{BlowStrength, Embouchure, Trumpet, TrumpetDefinition, Valve, BFLAT_TRUMPET},
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

#[derive(Debug, Clone, Copy)]
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

        // TODO: make logic for detecting valve changes / changes in pot values and add them to the events list

        let mut events: Vec<TrumpetEvent, 4> = Vec::new();

        for (valve, (&current_state, &last_state)) in current_state
            .valves()
            .iter()
            .zip(self.last_trumpet_state.valves().iter())
            .enumerate()
        {
            let event = if !last_state && current_state {
                TrumpetEvent::ValveDown(Valve::from(valve))
            } else if last_state && !current_state {
                TrumpetEvent::ValveUp(Valve::from(valve))
            } else {
                continue;
            };

            events
                .push(event)
                .ok()
                .expect("No more than three valves and 4 elements in events so should be fine");
        }

        // TODO: make const
        let pot_threshold: I1F15 = I1F15::from_num(100. / 4096.);

        let enough_change = |last: I1F15, current: I1F15| (last - current).abs() > pot_threshold;

        // TODO: one of these can in theory be dropped and may cause weirdness
        if enough_change(
            self.last_trumpet_state.blowstrength,
            current_state.blowstrength,
        ) {
            events
                .push(TrumpetEvent::BlowStrengthChange(current_state.blowstrength))
                .ok();
        }

        if enough_change(self.last_trumpet_state.embouchure, current_state.embouchure) {
            events
                .push(TrumpetEvent::EmbouchureChange(current_state.blowstrength))
                .ok();
        }

        self.last_trumpet_state = current_state;
    }
}

// TODO: tests are possible using this library setup, e.g. fuzzers for checking panic-free-ness

pub struct TrumpetInterface<FIFO, INPUTS> {
    fifo: FIFO,
    inputs: TrumpetInputs<INPUTS>,
    trumpet: Trumpet,
}

impl<FIFO: Fifo, INPUTS: Inputs> TrumpetInterface<FIFO, INPUTS> {
    pub fn new(io: IO<FIFO, INPUTS>) -> Self {
        Self {
            fifo: io.fifo,
            inputs: TrumpetInputs::new(io.inputs),
            trumpet: Trumpet::new(BFLAT_TRUMPET),
        }
    }

    pub fn run(&mut self) {
        self.inputs.update_events();
        self.trumpet.update(self.inputs.events());
        // based on these events, update the model, and based on what the model returns, send commands to the synth
    }
}
