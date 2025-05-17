use embedded_hal::digital::v2::InputPin;
use rp2040_hal::gpio::{bank0::*, FunctionSioInput, Pin, PullUp};

use trumpet_synth::io::Fifo;

pub struct SioFifo(pub rp2040_hal::sio::SioFifo);

impl Fifo for SioFifo {
    fn write(&mut self, value: u32) {
        self.0.write(value);
    }
}
