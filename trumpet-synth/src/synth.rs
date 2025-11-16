use fixed::types::I1F15;
use rytmos_synth::{
    commands::CommandMessage,
    effect::{
        lpf::{LowPassFilter, LowPassFilterSettings},
        Effect,
    },
    synth::{
        sawtooth::{SawtoothSynth, SawtoothSynthSettings},
        Synth,
    },
};
use serde::{Deserialize, Serialize};

pub fn create() -> TrumpetSynth {
    TrumpetSynth::make(0x0, ())
}

pub struct TrumpetSynth {
    sawtooth: SawtoothSynth,
    lpf: LowPassFilter,
}

impl Synth for TrumpetSynth {
    type Settings = ();

    fn make(address: u32, (): Self::Settings) -> Self
    where
        Self: Sized,
    {
        Self {
            sawtooth: SawtoothSynth::make(address, SawtoothSynthSettings {}),
            lpf: LowPassFilter::new(LowPassFilterSettings {
                alpha: I1F15::unwrapped_from_str("0.01"), // higher = less filter
            }),
        }
    }

    fn configure(&mut self, (): Self::Settings) {}

    fn play(&mut self, _note: rytmos_engrave::staff::Note, _velocity: fixed::types::U4F4) {
        // Do nothing, trumpet synth only supports freq()
    }

    fn freq(&mut self, freq: fixed::types::U12F4) {
        self.sawtooth.freq(freq);
    }

    fn attack(&mut self, attack: fixed::types::U4F4) {
        self.sawtooth.attack(attack);
    }

    fn next(&mut self) -> fixed::types::I1F15 {
        let sample = self.sawtooth.next();
        self.lpf.next(sample)
    }

    fn run_command(&mut self, command: rytmos_synth::commands::Command) {
        self.sawtooth.run_command(command);

        if let CommandMessage::Reconfigure(command_serialized) = command.message {
            let command = TrumpetSynthCommand::deserialize(command_serialized);
        }
    }

    fn address(&self) -> u32 {
        0
    }
}

pub(crate) enum TrumpetSynthCommand {
    FilterAlpha(I1F15),
}

impl TrumpetSynthCommand {
    fn deserialize(command_serialized: u32) -> Option<Self> {
        TODO
    }
}
