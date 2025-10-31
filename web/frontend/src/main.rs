//! Run with:
//!
//! ```sh
//! dx serve --platform web
//! ```
#![allow(non_snake_case)]

use core::borrow::BorrowMut;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use dioxus::prelude::*;
#[allow(unused_imports)]
use tracing::info;
use trumpet_synth::interface::TrumpetInterface;
use trumpet_synth::io::IO;
use trumpet_synth_web::io::{WebFifo, WebInputs};
use wasm_bindgen::closure::Closure;
use wasm_bindgen_futures::JsFuture;
use web_sys::js_sys::Array;
use web_sys::wasm_bindgen::JsCast;
use web_sys::{
    window, AudioContext, AudioWorkletNode, AudioWorkletNodeOptions, Request, RequestInit, Response,
};

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
        if !self.is_audio_initialized() {
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

const SLIDER_LENGTH: usize = 7;

/// Takes keydown/keyup events, modifies the input signals to what those button presses correspond to
struct InputBehavior {
    signals: WebInputs,

    emb_slider: SpringSlider<SLIDER_LENGTH>,
    blow_slider: SpringSlider<SLIDER_LENGTH>,
}

impl InputBehavior {
    const EMBOUCHURES: [&'static str; SLIDER_LENGTH] = ["A", "S", "D", "F", "G", "H", "J"];
    const BLOWSTRENGTHS: [&'static str; SLIDER_LENGTH] = ["Z", "X", "C", "V", "B", "N", "M"];

    fn new(signals: WebInputs) -> Self {
        Self {
            signals,
            emb_slider: SpringSlider::new(),
            blow_slider: SpringSlider::new(),
        }
    }

    fn update_internal_representation_of_input(
        &mut self,
        event: web_sys::KeyboardEvent,
        bool_to_set: bool,
    ) {
        // keys comma, period, slash for the valves
        // embouchure slider is asdfghj
        // blowstrength slider is zxcvbnm
        // TODO: map embouchure and blowstrength to xbox controller joystick
        match event.code().as_str() {
            "Comma" => self.signals.first_valve_signal.set(bool_to_set),
            "Period" => self.signals.second_valve_signal.set(bool_to_set),
            "Slash" => self.signals.third_valve_signal.set(bool_to_set),
            "Space" => self.signals.blow_signal.set(bool_to_set),
            _ => (),
        }

        if event.code().starts_with("Key") {
            let key = event.code();
            let key = key.replace("Key", "");
            if let Some(index) = Self::EMBOUCHURES.iter().position(|&k| key.eq(k)) {
                self.emb_slider.update_keys(index, bool_to_set);
            }

            if let Some(index) = Self::BLOWSTRENGTHS.iter().position(|&k| key.eq(k)) {
                self.blow_slider.update_keys(index, bool_to_set);
            }
        }
    }

    pub fn keyup(&mut self, event: web_sys::KeyboardEvent) {
        self.update_internal_representation_of_input(event, false);
    }

    pub fn keydown(&mut self, event: web_sys::KeyboardEvent) {
        self.update_internal_representation_of_input(event, true);
    }

    pub fn update(&mut self, dt: u64) {
        self.signals
            .embouchure_signal
            .set(self.emb_slider.update(dt));
        self.signals
            .blowstrength_signal
            .set(self.blow_slider.update(dt));
    }
}

struct SpringSlider<const N: usize> {
    keys: [bool; N],
    state: f64,
    speed: f64,
}

impl<const N: usize> SpringSlider<N> {
    pub fn new() -> Self {
        Self {
            keys: [false; N],
            state: 0.,
            speed: 0.,
        }
    }

    pub fn update_keys(&mut self, index: usize, bool_to_set: bool) {
        if index < N {
            self.keys[index] = bool_to_set
        }
    }

    pub fn update(&mut self, dt: u64) -> f64 {
        let embouchure_setting = self
            .keys
            .iter()
            .rposition(|&x| x)
            .map(|i| i + 1)
            .unwrap_or(0);

        let dt = dt as f64 * 1e-3;

        match embouchure_setting {
            0 => {
                self.state -= dt * self.speed;
                self.speed += dt * 10.;
            }
            _ => {
                self.state = (embouchure_setting as f64 / N as f64) * 0.99;
                self.speed = 10.;
            }
        }

        self.state.clamp(0., 0.999)
    }
}

fn app() -> Element {
    let mut audio_setup = AudioSetup::new();
    audio_setup.setup_wasm();

    let first_valve_signal = use_signal(|| false);
    let second_valve_signal = use_signal(|| false);
    let third_valve_signal = use_signal(|| false);
    let blow_signal = use_signal(|| false);
    let embouchure_signal = use_signal(|| 0.0);
    let blowstrength_signal = use_signal(|| 0.0);

    let inputs = WebInputs {
        first_valve_signal,
        second_valve_signal,
        third_valve_signal,
        blow_signal,
        embouchure_signal,
        blowstrength_signal,
    };

    let input_behavior = Arc::new(Mutex::new(InputBehavior::new(inputs.clone())));

    use_future({
        let inputs = inputs.clone();
        let input_behavior = Arc::clone(&input_behavior);
        move || {
            let inputs = inputs.clone();
            let input_behavior = Arc::clone(&input_behavior);
            async move {
                let io = IO {
                    fifo: WebFifo::new(audio_setup.node_signal),
                    inputs,
                };

                let mut interface = TrumpetInterface::new(io, 1);

                const MILLIS_PER_ITER: u64 = 10;
                let mut dt = MILLIS_PER_ITER;

                loop {
                    interface.run();

                    // If we can't lock, just skip this update, don't block
                    if let Ok(mut b) = input_behavior.try_lock() {
                        b.update(dt);
                        dt = MILLIS_PER_ITER;
                    } else {
                        dt += MILLIS_PER_ITER;
                    }

                    async_std::task::sleep(Duration::from_millis(MILLIS_PER_ITER)).await
                }
            }
        }
    });

    use_future({
        let input_behavior = Arc::clone(&input_behavior);
        move || {
            let input_behavior = Arc::clone(&input_behavior);
            async move {
                let keydown_closure: Closure<dyn FnMut(web_sys::KeyboardEvent)> = Closure::new({
                    let input_behavior = Arc::clone(&input_behavior);
                    move |event: web_sys::KeyboardEvent| {
                        let mut b = input_behavior.lock().unwrap();
                        b.keydown(event);
                    }
                });

                let keyup_closure: Closure<dyn FnMut(web_sys::KeyboardEvent)> = Closure::new({
                    let input_behavior = Arc::clone(&input_behavior);
                    move |event: web_sys::KeyboardEvent| {
                        let mut b = input_behavior.lock().unwrap();
                        b.keyup(event);
                    }
                });

                let window = window().unwrap();
                window
                    .add_event_listener_with_callback(
                        "keydown",
                        keydown_closure.as_ref().unchecked_ref(),
                    )
                    .unwrap();
                window
                    .add_event_listener_with_callback(
                        "keyup",
                        keyup_closure.as_ref().unchecked_ref(),
                    )
                    .unwrap();

                keydown_closure.forget();
                keyup_closure.forget();
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

                div {
                    style: "display: flex",
                    {valve_button(inputs.first_valve_signal)}
                    {valve_button(inputs.second_valve_signal)}
                    {valve_button(inputs.third_valve_signal)}
                }

                {slider(30., inputs.embouchure_signal, "red")}
                {slider(30., inputs.blowstrength_signal, "blue")}
                {valve_button(inputs.blow_signal)}

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

fn valve_button(valve: Signal<bool>) -> Element {
    let class = if *valve.read() {
        "valve-down"
    } else {
        "valve-up"
    };

    rsx! {
        div {
            class: format!("valve {}", class),
        }
    }
}

fn slider(width_scale: f64, value: Signal<f64>, color: &str) -> Element {
    let width = *value.read() * width_scale + 0.5;
    rsx! {
        div {
            class: format!("slider"),
            style: format!("width: {}vw; background-color: {}", width, color),
        }
    }
}

fn main() {
    dioxus::launch(app);
}
