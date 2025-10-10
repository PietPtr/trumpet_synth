use trumpet_synth::{
    interface::TrumpetInterface,
    io::{Fifo, Inputs, IO},
};

struct TestFifo {}

impl Fifo for TestFifo {
    fn write(&mut self, value: u32) {
        todo!()
    }
}

struct TestInputs {}

impl Inputs for TestInputs {
    fn valve(&mut self, valve: trumpet_synth::trumpet::Valve) -> bool {
        todo!()
    }

    fn blow(&mut self) -> bool {
        todo!()
    }

    fn embouchure(&mut self) -> trumpet_synth::trumpet::Embouchure {
        todo!()
    }

    fn blowstrength(&mut self) -> trumpet_synth::trumpet::BlowStrength {
        todo!()
    }
}

#[test]
fn test_trumpet_frequency() {
    let mut interface = TrumpetInterface::new(IO {
        fifo: TestFifo {},
        inputs: TestInputs {},
    });

    interface.run();

    // TODO: resolve todo's in test impls of hardware, make mutexes for state?
    // TODO: make sure the inputs change between calls to run and observe behaviour
}
