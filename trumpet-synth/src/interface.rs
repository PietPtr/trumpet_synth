use common::debouncer::Debouncer;
use fixed::types::U0F16;
use heapless::Vec;

use crate::{
    io::{Fifo, Inputs, TrumpetInputState, IO},
    trumpet::{BlowStrength, Embouchure, Trumpet, Valve, BFLAT_TRUMPET},
};

// TODO: different place for some of these structs/impls?

#[derive(Debug, Clone, Copy)]
pub enum TrumpetEvent {
    BlowUp,
    BlowDown,
    ValveUp(Valve),
    ValveDown(Valve),
    EmbouchureChange(Embouchure),
    BlowStrengthChange(BlowStrength),
}

#[cfg(feature = "defmt")]
impl defmt::Format for TrumpetEvent {
    fn format(&self, fmt: defmt::Formatter) {
        match self {
            TrumpetEvent::BlowUp => defmt::write!(fmt, "TrumpetEvent::BlowUp"),
            TrumpetEvent::BlowDown => defmt::write!(fmt, "TrumpetEvent::BlowDown"),
            TrumpetEvent::ValveUp(v) => {
                defmt::write!(fmt, "TrumpetEvent::ValveUp({:?})", Into::<usize>::into(*v))
            }
            TrumpetEvent::ValveDown(v) => {
                defmt::write!(
                    fmt,
                    "TrumpetEvent::ValveDown({:?})",
                    Into::<usize>::into(*v)
                )
            }
            TrumpetEvent::EmbouchureChange(e) => {
                defmt::write!(fmt, "TrumpetEvent::EmbouchureChange({:?})", e.to_bits())
            }
            TrumpetEvent::BlowStrengthChange(b) => {
                defmt::write!(fmt, "TrumpetEvent::BlowStrengthChange({:?})", b.to_bits())
            }
        }
    }
}
/// Abstraction over raw input readings, takes care of e.g. debouncing and
/// rescaling the potentiometer values.
pub struct TrumpetInputs<INPUTS> {
    inputs: INPUTS,
    valve_debouncers: [Debouncer; 3],
    blow_debouncer: Debouncer,
    events: Vec<TrumpetEvent, 8>,
    last_trumpet_state: TrumpetInputState,
}

impl<INPUTS: Inputs> TrumpetInputs<INPUTS> {
    pub fn new(inputs: INPUTS, debounce_time: u32) -> Self {
        Self {
            inputs,
            valve_debouncers: [Debouncer::new(debounce_time); 3],
            blow_debouncer: Debouncer::new(debounce_time),
            events: Vec::new(),
            last_trumpet_state: TrumpetInputState::default(),
        }
    }

    fn update_debouncers(&mut self, state: TrumpetInputState) {
        for (id, debouncer) in self.valve_debouncers.iter_mut().enumerate() {
            debouncer.update(state.valve(Valve::from(id)));
        }

        self.blow_debouncer.update(state.blow)
    }

    pub fn events(&self) -> &[TrumpetEvent] {
        &self.events
    }

    fn debouncer_is_high(debouncer: Option<&Debouncer>) -> bool {
        let Some(debouncer) = debouncer else {
            panic!("No debouncer found.");
        };

        debouncer.is_high().unwrap_or(false)
    }

    fn valve_debouncer_is_high(&self, valve: Valve) -> bool {
        let debouncer_index = valve as usize;
        Self::debouncer_is_high(self.valve_debouncers.get(debouncer_index))
    }

    fn blow_debouncer_is_high(&self) -> bool {
        Self::debouncer_is_high(Some(&self.blow_debouncer))
    }

    pub fn update_events(&mut self) {
        let mut current_state = TrumpetInputState::read_from(&mut self.inputs);

        self.update_debouncers(current_state);

        current_state.first = self.valve_debouncer_is_high(Valve::First);
        current_state.second = self.valve_debouncer_is_high(Valve::Second);
        current_state.third = self.valve_debouncer_is_high(Valve::Third);
        current_state.blow = self.blow_debouncer_is_high();

        self.events.clear();

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

            self.events.push(event).ok().expect("Valve event dropped");
        }

        let event = if !self.last_trumpet_state.blow && current_state.blow {
            Some(TrumpetEvent::BlowDown)
        } else if self.last_trumpet_state.blow && !current_state.blow {
            Some(TrumpetEvent::BlowUp)
        } else {
            None
        };

        if let Some(event) = event {
            self.events.push(event).ok().expect("Blow event dropped");
        }

        // TODO: make const
        let pot_threshold: U0F16 = U0F16::from_num(20. / 65536.);

        let enough_change = |last: U0F16, current: U0F16| {
            ((last.saturating_sub(current)) > pot_threshold)
                || (current.saturating_sub(last) > pot_threshold)
        };

        if enough_change(
            self.last_trumpet_state.blowstrength,
            current_state.blowstrength,
        ) {
            self.events
                .push(TrumpetEvent::BlowStrengthChange(current_state.blowstrength))
                .ok()
                .expect("Blowstrength event dropped");
        } else {
        }

        if enough_change(self.last_trumpet_state.embouchure, current_state.embouchure) {
            self.events
                .push(TrumpetEvent::EmbouchureChange(current_state.embouchure))
                .ok()
                .expect("Embouchure event dropped");
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
    pub fn new(io: IO<FIFO, INPUTS>, debounce_time: u32) -> Self {
        Self {
            fifo: io.fifo,
            inputs: TrumpetInputs::new(io.inputs, debounce_time),
            trumpet: Trumpet::new(BFLAT_TRUMPET),
        }
    }

    pub fn run(&mut self) {
        self.inputs.update_events();
        let commands = self.trumpet.update(self.inputs.events());

        if self.inputs.events().len() > 0 {
            // defmt::info!("events: {:?}", self.inputs.events());
        }
        if commands.len() > 0 {
            // defmt::info!("commands: {:?}", commands.len());
        }

        for command in commands {
            self.fifo.write(command.serialize())
        }
    }
}
