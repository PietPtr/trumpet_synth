use dioxus::signals::{Readable, Signal};
use fixed::types::I1F15;
use trumpet_synth::{io, trumpet::Valves};
use web_sys::{wasm_bindgen::JsValue, AudioWorkletNode};

// TODO: common between polypicophonic and trumpet_synth
pub struct WebFifo {
    node: Signal<Option<AudioWorkletNode>>,
}

impl WebFifo {
    pub fn new(node: Signal<Option<AudioWorkletNode>>) -> Self {
        Self { node }
    }
}

impl io::Fifo for WebFifo {
    fn write(&mut self, value: u32) {
        let binding = self.node.read();
        let node = binding.as_ref().unwrap();
        node.port()
            .unwrap()
            .post_message(&JsValue::from_f64(value as f64))
            .unwrap();
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WebInputs {
    pub first_valve_signal: Signal<bool>,
    pub second_valve_signal: Signal<bool>,
    pub third_valve_signal: Signal<bool>,
    pub embouchure_signal: Signal<f64>,
    pub blowstrength_signal: Signal<f64>,
}

impl io::Inputs for WebInputs {
    fn valves(&mut self) -> trumpet_synth::trumpet::Valves {
        Valves {
            first: (*self.first_valve_signal.read()).into(),
            second: (*self.second_valve_signal.read()).into(),
            third: (*self.third_valve_signal.read()).into(),
        }
    }

    fn embouchure(&mut self) -> trumpet_synth::trumpet::Embouchure {
        I1F15::from_num(*self.embouchure_signal.read())
    }

    fn blowstrength(&mut self) -> trumpet_synth::trumpet::BlowStrength {
        I1F15::from_num(*self.blowstrength_signal.read())
    }
}
