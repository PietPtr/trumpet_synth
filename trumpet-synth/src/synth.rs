use rytmos_synth::synth::{
    sawtooth::{SawtoothSynth, SawtoothSynthSettings},
    Synth,
};

pub type TrumpetSynth = SawtoothSynth;

pub fn create() -> TrumpetSynth {
    SawtoothSynth::make(0x0, SawtoothSynthSettings {})
}
