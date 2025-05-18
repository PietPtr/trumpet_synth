use crate::trumpet::{BlowStrength, Embouchure, Valves};

pub struct IO<FIFO, INPUTS> {
    pub fifo: FIFO,
    pub inputs: INPUTS,
}

impl<FIFO, INPUTS> IO<FIFO, INPUTS>
where
    FIFO: Fifo,
    INPUTS: Inputs,
{
    pub fn new(fifo: FIFO, inputs: INPUTS) -> Self {
        Self { fifo, inputs }
    }
}

pub trait Fifo {
    fn write(&mut self, value: u32);
}

pub trait Inputs {
    fn valves(&mut self) -> Valves;
    fn embouchure(&mut self) -> Embouchure;
    fn blowstrength(&mut self) -> BlowStrength;
}
