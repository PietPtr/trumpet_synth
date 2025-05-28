use dioxus::signals::{Readable, Signal};
use fixed::types::U0F16;
use trumpet_synth::{
    io,
    trumpet::{BlowStrength, Embouchure, Valve},
};
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
    fn valve(&mut self, valve: Valve) -> bool {
        match valve {
            Valve::First => (*self.first_valve_signal.read()).into(),
            Valve::Second => (*self.second_valve_signal.read()).into(),
            Valve::Third => (*self.third_valve_signal.read()).into(),
        }
    }

    fn embouchure(&mut self) -> Embouchure {
        U0F16::from_num(*self.embouchure_signal.read())
    }

    fn blowstrength(&mut self) -> BlowStrength {
        U0F16::from_num(*self.blowstrength_signal.read())
    }
}
