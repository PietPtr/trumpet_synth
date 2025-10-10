use crate::trumpet::{BlowStrength, Embouchure, Valve};

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
    fn valve(&mut self, valve: Valve) -> bool;
    fn blow(&mut self) -> bool;
    fn embouchure(&mut self) -> Embouchure;
    fn blowstrength(&mut self) -> BlowStrength;
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct TrumpetInputState {
    pub first: bool,
    pub second: bool,
    pub third: bool,
    pub blow: bool,
    pub embouchure: Embouchure,
    pub blowstrength: BlowStrength,
}

impl TrumpetInputState {
    pub fn read_from<I: Inputs>(inputs: &mut I) -> Self {
        Self {
            first: inputs.valve(Valve::First),
            second: inputs.valve(Valve::Second),
            third: inputs.valve(Valve::Third),
            blow: inputs.blow(),
            embouchure: inputs.embouchure(),
            blowstrength: inputs.blowstrength(),
        }
    }

    pub(crate) fn valves(&self) -> [bool; 3] {
        [self.first, self.second, self.third]
    }

    pub(crate) fn valve(&self, id: Valve) -> bool {
        match id {
            Valve::First => self.first,
            Valve::Second => self.second,
            Valve::Third => self.third,
        }
    }
}
