use embedded_hal::adc::OneShot;
use embedded_hal::digital::v2::InputPin;
use fixed::types::U0F16;
use rp2040_hal::{
    adc::AdcPin,
    gpio::{bank0::*, DynFunction, DynPinId, FunctionSioInput, Pin, PullDown, PullUp},
    Adc,
};

use trumpet_synth::io::{self};

pub struct SioFifo(pub rp2040_hal::sio::SioFifo);

impl io::Fifo for SioFifo {
    fn write(&mut self, value: u32) {
        self.0.write(value);
    }
}

pub struct Rp2040Inputs {
    pub valve_pins: [Pin<DynPinId, FunctionSioInput, PullUp>; 3],
    pub blow_pin: Pin<Gpio0, FunctionSioInput, PullUp>,
    pub adc: Adc,
    pub adc_pins: [AdcPin<Pin<DynPinId, DynFunction, PullDown>>; 2],
}

fn convert_adc_value(read: u16) -> U0F16 {
    U0F16::from_bits(read << 4)
}

impl io::Inputs for Rp2040Inputs {
    fn valve(&mut self, valve: trumpet_synth::trumpet::Valve) -> bool {
        self.valve_pins[valve as usize].is_low().unwrap()
    }

    fn blow(&mut self) -> bool {
        self.blow_pin.is_low().unwrap()
    }

    fn embouchure(&mut self) -> trumpet_synth::trumpet::Embouchure {
        let adc0_read: u16 = self.adc.read(&mut self.adc_pins[1]).unwrap();
        convert_adc_value(adc0_read)
    }

    fn blowstrength(&mut self) -> trumpet_synth::trumpet::BlowStrength {
        let adc1_read: u16 = self.adc.read(&mut self.adc_pins[0]).unwrap();
        convert_adc_value(adc1_read)
    }
}
