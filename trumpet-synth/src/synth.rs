use fixed::types::I1F15;
use rytmos_synth::{
    effect::{
        linear_decay::{LinearDecay, LinearDecaySettings},
        lpf::{LowPassFilter, LowPassFilterSettings},
    },
    synth::{
        composed::{
            polyphonic::PolyphonicSynth,
            synth_with_effects::{SynthWithEffect, SynthWithEffectSettings},
        },
        sine::{SineSynth, SineSynthSettings},
        Synth,
    },
};

type WaveSynth = SineSynth;
type TheSynth = SynthWithEffect<SynthWithEffect<WaveSynth, LinearDecay>, LowPassFilter>;
pub type PolypicophonicSynth = PolyphonicSynth<4, TheSynth>;

pub fn create() -> PolypicophonicSynth {
    let settings =
        SynthWithEffectSettings::<SynthWithEffect<WaveSynth, LinearDecay>, LowPassFilter> {
            synth: SynthWithEffectSettings::<WaveSynth, LinearDecay> {
                synth: SineSynthSettings::default(),
                effect: LinearDecaySettings {
                    decay: I1F15::from_num(0.0005),
                    decay_every: 32,
                },
            },
            effect: LowPassFilterSettings {
                alpha: I1F15::from_num(0.05),
            },
        };

    PolyphonicSynth::<4, TheSynth>::make(0, settings)
}
