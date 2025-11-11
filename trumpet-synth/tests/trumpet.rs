use plotters::prelude::*;
use std::{error::Error, process::Command};

use trumpet_synth::{
    interface::TrumpetEvent,
    trumpet::{BlowStrength, Embouchure, Trumpet, BFLAT_TRUMPET},
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

#[test]
fn plot_embouchure_to_frequency() {
    let mut trumpet = Trumpet::new(BFLAT_TRUMPET);
    trumpet.update(&[
        TrumpetEvent::BlowDown,
        TrumpetEvent::BlowStrengthChange(BlowStrength::from_num(0.9)),
    ]);

    let mut result: Vec<(f64, f64)> = Vec::new();
    for i in (0..u16::MAX).step_by(1 << 4) {
        let embouchure = Embouchure::from_bits(i);

        trumpet.update(&[TrumpetEvent::EmbouchureChange(embouchure)]);

        let frequency = trumpet.frequency().map(|n| n.to_num()).unwrap_or(0.);
        let embouchure = embouchure.to_num();
        result.push((embouchure, frequency))
    }

    plot(result, "Frequency", "Embouchure").unwrap();
}

fn plot(data: Vec<(f64, f64)>, y_label: &str, x_label: &str) -> Result<(), Box<dyn Error>> {
    let mut binding = Command::new("cp");
    let cmd = binding.arg("plot.png").arg("old-plot.png");
    cmd.output().unwrap();

    let root = BitMapBackend::new("plot.png", (900, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    let (x_min, x_max) = data
        .iter()
        .map(|(x, _)| *x)
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), x| {
            (a.min(x), b.max(x))
        });
    let (y_min, _y_max) = data
        .iter()
        .map(|(_, y)| *y)
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), y| {
            (a.min(y), b.max(y))
        });

    let mut chart = ChartBuilder::on(&root)
        .margin(30)
        .caption(
            format!("{:?}", std::time::SystemTime::now()),
            ("sans-serif", 15),
        )
        .x_label_area_size(80)
        .y_label_area_size(80)
        .build_cartesian_2d(x_min..x_max, 0.0..2000.0)?;

    chart
        .configure_mesh()
        .x_desc(x_label)
        .y_desc(y_label)
        .draw()?;
    chart.draw_series(LineSeries::new(data.clone(), &BLUE))?;

    Ok(())
}
