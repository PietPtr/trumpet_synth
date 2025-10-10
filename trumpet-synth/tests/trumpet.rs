use trumpet_synth::{
    interface::TrumpetEvent,
    trumpet::{self, BlowStrength, Embouchure, Trumpet, BFLAT_TRUMPET},
};

#[test]
fn test_trumpet_frequencies() {
    let mut trumpet = Trumpet::new(BFLAT_TRUMPET);

    trumpet.update(&[
        TrumpetEvent::BlowDown,
        TrumpetEvent::BlowStrengthChange(BlowStrength::from_num(0.8)),
        TrumpetEvent::EmbouchureChange(Embouchure::from_num(0.23)),
    ]);

    dbg!(
        trumpet.state.overtone(),
        trumpet.state.tube_length(&BFLAT_TRUMPET),
        trumpet.state.volume(),
        trumpet.frequency(),
    );
}
