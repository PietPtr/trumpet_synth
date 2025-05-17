use dioxus::signals::{Readable, Signal};
use trumpet_synth::io;
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
