//! Run with:
//!
//! ```sh
//! dx serve --platform web
//! ```
#![allow(non_snake_case)]

use core::borrow::BorrowMut;
use std::sync::OnceLock;
use std::time::Duration;

use dioxus::prelude::*;
use trumpet_synth::interface::TrumpetInterface;
use trumpet_synth::io::IO;
use trumpet_synth_web::io::{WebFifo, WebInputs};
// use trumpet_synth_web::io::{}; // TODO: define IO
use wasm_bindgen_futures::JsFuture;
use web_sys::js_sys::Array;
use web_sys::wasm_bindgen::JsCast;
use web_sys::{
    window, AudioContext, AudioWorkletNode, AudioWorkletNodeOptions, Request, RequestInit, Response,
};

// TODO: use_effect or something?
static ONCE: OnceLock<()> = OnceLock::new();

pub struct AudioSetup {
    pub node_signal: Signal<Option<AudioWorkletNode>>,
    pub ctx_signal: Signal<Option<AudioContext>>,
    pub is_audio_initialized_signal: Signal<bool>,
}

// TODO: move to library
impl AudioSetup {
    pub fn new() -> Self {
        Self {
            node_signal: use_signal(|| None),
            ctx_signal: use_signal(|| None),
            is_audio_initialized_signal: use_signal(|| false),
        }
    }

    pub fn setup_wasm(&self) {
        let mut node_signal = self.node_signal;
        let mut ctx_signal = self.ctx_signal;
        use_future(move || async move {
            let ctx = AudioContext::new().unwrap();

            JsFuture::from(
                ctx.audio_worklet()
                    .unwrap()
                    .add_module(&asset!("/assets/wasm_audio.js").to_string())
                    .unwrap(),
            )
            .await
            .unwrap();

            let options = RequestInit::new();
            options.set_method("GET");
            let request = Request::new_with_str_and_init(
                &asset!("/assets/wasm_audio_bg.wasm").to_string(),
                &options,
            )
            .unwrap();

            let window = window().unwrap();
            let response = JsFuture::from(window.fetch_with_request(&request))
                .await
                .unwrap()
                .unchecked_into::<Response>();

            let array_buffer = JsFuture::from(response.array_buffer().unwrap())
                .await
                .unwrap();

            let options = AudioWorkletNodeOptions::new();
            options.set_processor_options(Some(&Array::of1(&array_buffer)));

            let node = AudioWorkletNode::new_with_options(&ctx, "my-processor", &options).unwrap();
            node.connect_with_audio_node(&ctx.destination()).unwrap();

            node_signal.set(Some(node));
            ctx_signal.set(Some(ctx));
        });
    }

    pub fn initialize_audio(&mut self) {
        if !*self.is_audio_initialized_signal.read() {
            drop(
                self.ctx_signal
                    .borrow_mut()
                    .as_mut()
                    .unwrap()
                    .resume()
                    .unwrap(),
            );
        }

        self.is_audio_initialized_signal.set(true);
    }

    pub fn is_audio_initialized(&self) -> bool {
        *self.is_audio_initialized_signal.read()
    }
}

fn app() -> Element {
    let mut audio_setup = AudioSetup::new();
    audio_setup.setup_wasm();

    let first_valve_signal = use_signal(|| false);
    let second_valve_signal = use_signal(|| false);
    let third_valve_signal = use_signal(|| false);
    let embouchure_signal = use_signal(|| 0.0);
    let blowstrength_signal = use_signal(|| 0.0);

    let inputs = WebInputs {
        first_valve_signal,
        second_valve_signal,
        third_valve_signal,
        embouchure_signal,
        blowstrength_signal,
    };

    use_future({
        let inputs = inputs.clone();
        move || {
            let inputs = inputs.clone();
            async move {
                let io = IO {
                    fifo: WebFifo::new(audio_setup.node_signal),
                    inputs,
                };

                let mut interface = TrumpetInterface::new(io);

                loop {
                    // for _ in 0..10 {
                    interface.run();

                    async_std::task::sleep(Duration::from_millis(10)).await
                }
            }
        }
    });

    rsx! {
        div {
            class: "content",
            tabindex: 0,

            document::Link { href: asset!("/assets/stylesheet.css"), rel: "stylesheet" }
            // document::Link { rel: "icon", type: "image/png", href: asset!("/assets/icon.png") }
            document::Title { "Trumpet Synth 🎺" }

            div {
                class: "header",
                h1 {
                    "Trumpet Synth"
                }

                {valve_button(inputs.first_valve_signal)}
                {valve_button(inputs.second_valve_signal)}
                {valve_button(inputs.third_valve_signal)}

                if !audio_setup.is_audio_initialized() {
                    button {
                        class: "start-button",
                        onclick: move |_| audio_setup.initialize_audio(),
                        "Start Audio Engine"
                    }
                }
            }

        }
    }
}

fn valve_button(mut valve: Signal<bool>) -> Element {
    rsx! {
        div {
            class: "",
            button {
                class: "todo",
                onmousedown: move |_| valve.set(true),
                ontouchstart: move |_| valve.set(true),
                onmouseup: move |_| valve.set(false),
                ontouchend: move |_| valve.set(false),
                onmouseleave: move |_| valve.set(false),
                "( )"
            }
        }
    }
}

fn main() {
    dioxus::launch(app);
}
