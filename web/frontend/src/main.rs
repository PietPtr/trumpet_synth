//! Run with:
//!
//! ```sh
//! dx serve --platform web
//! ```
#![allow(non_snake_case)]

use core::borrow::BorrowMut;
use std::sync::OnceLock;
use std::time::Duration;
use std::{convert::TryFrom, iter::Iterator};

use dioxus::prelude::*;
use trumpet_synth::io::IO;
use trumpet_synth_web::io::WebFifo;
// use trumpet_synth_web::io::{}; // TODO: define IO
use wasm_bindgen_futures::JsFuture;
use web_sys::js_sys::Array;
use web_sys::wasm_bindgen::prelude::Closure;
use web_sys::wasm_bindgen::JsCast;
use web_sys::{
    window, AudioContext, AudioWorkletNode, AudioWorkletNodeOptions, Request, RequestInit, Response,
};

// TODO: use_effect or something?
static ONCE: OnceLock<()> = OnceLock::new();

fn app() -> Element {
    let mut ctx_signal: Signal<Option<AudioContext>> = use_signal(|| None);
    let mut node_signal: Signal<Option<AudioWorkletNode>> = use_signal(|| None);

    use_future({
        move || {
            async move {
                let io = IO {
                    fifo: WebFifo::new(node_signal),
                };

                // let mut interface = SandboxInterface::new(io);

                loop {
                    // for _ in 0..10 {
                    // interface.run();
                    // TODO: define trumpet's run loop

                    async_std::task::sleep(Duration::from_millis(10)).await
                }
            }
        }
    });

    // TODO: common?
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

    // TODO: common?
    let mut is_audio_initialized = use_signal(|| false);

    let mut initialize_audio = move || {
        if !is_audio_initialized() {
            drop(ctx_signal.borrow_mut().as_mut().unwrap().resume().unwrap());
        }

        is_audio_initialized.set(true);
    };

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

                if !is_audio_initialized() {
                    button {
                        class: "start-button",
                        onclick: move |_| initialize_audio(),
                        "Start Audio Engine"
                    }
                }
            }

        }
    }
}

fn main() {
    dioxus::launch(app);
}
