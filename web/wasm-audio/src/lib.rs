use core::convert::{AsRef, Into};
use std::sync::{Mutex, OnceLock};

use js_sys::{Array, Float32Array, Object};
use log::Level;
use rytmos_synth::{commands::Command, synth::Synth};
use wasm_bindgen::prelude::*;
use web_sys::{MessageEvent, MessagePort};

static SYNTH: OnceLock<Mutex<trumpet_synth::synth::PolypicophonicSynth>> = OnceLock::new();

#[wasm_bindgen]
pub fn init_logging() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(Level::Debug).unwrap();

    log::info!("Initialized synth logging and panic handler.");

    let synth = trumpet_synth::synth::create();

    if SYNTH.set(Mutex::new(synth)).is_err() {
        panic!("Cannot set SYNTH");
    }
}

#[wasm_bindgen]
pub struct Processor {
    _port: MessagePort,
    _message_closure: Closure<dyn Fn(MessageEvent)>,
}

// TODO: make generic over S: Synth?
#[wasm_bindgen]
impl Processor {
    #[wasm_bindgen(constructor)]
    pub fn new(port: MessagePort) -> Self {
        let message_closure = Closure::new(|event: MessageEvent| {
            let mut synth = SYNTH.get().unwrap().lock().unwrap();
            let Some(serialized) = event.data().as_f64() else {
                log::error!("Event data not convertible to f64: {:?}", event.data());
                return;
            };

            let Some(command) = Command::deserialize(serialized as u32) else {
                log::error!("Deserialization failure of {}", serialized as u32);
                return;
            };

            synth.run_command(command)
        });

        port.add_event_listener_with_callback("message", message_closure.as_ref().unchecked_ref())
            .unwrap();

        log::info!("Created Synth");

        Self {
            _port: port,
            _message_closure: message_closure,
        }
    }

    #[wasm_bindgen]
    pub fn process(&mut self, _inputs: Array, outputs: Array, _parameters: Object) {
        let output = outputs.get(0).unchecked_into::<Array>();
        let channel = output.get(0).unchecked_into::<Float32Array>();

        let mut synth = SYNTH.get().unwrap().lock().unwrap();

        for j in 0..channel.length() / 2 {
            let sample = synth.next();
            // TODO: is this left rigth?
            channel.set_index(j * 2, f32::from(sample));
            channel.set_index(j * 2 + 1, f32::from(sample));
        }
    }
}
