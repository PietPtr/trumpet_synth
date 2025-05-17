use std::fs;
use wasm_audio::TEXT_DECODER_POLYFILL;
use wasm_audio::WORKLET;

const WASM_BASE_PATH: &str = "../wasm-audio/pkg/wasm_audio_trumpet_synth";

fn concat_wasm_resources(base_path: &str) {
    fs::copy(
        format!("{WASM_BASE_PATH}_bg.wasm"),
        "assets/wasm_audio_bg.wasm",
    )
    .unwrap();

    let js = std::fs::read_to_string(format!("{base_path}.js")).expect(
        "Cannot find wasm JS build product.\n Has `wasm-pack build --target web` been run?\n\n",
    );

    let concatted = format!("{TEXT_DECODER_POLYFILL}\n{js}\n{WORKLET}");

    fs::write("assets/wasm_audio.js", concatted).unwrap();
}

fn main() {
    concat_wasm_resources(WASM_BASE_PATH);
}
