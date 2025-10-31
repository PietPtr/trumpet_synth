use std::sync::{
    atomic::{AtomicBool, AtomicI16, AtomicU16, Ordering},
    Arc, Mutex,
};

use rytmos_synth::{commands::Command, synth::Synth};
use trumpet_synth::{
    interface::{TrumpetEvent, TrumpetInterface},
    io::{Fifo, Inputs, IO},
    trumpet::{BlowStrength, Embouchure, Valve},
};

struct TestFifo {
    fifo: Arc<Mutex<Vec<u32>>>,
}

impl Fifo for TestFifo {
    fn write(&mut self, value: u32) {
        let mut guard = self.fifo.lock().unwrap();
        guard.push(value);
    }
}

struct SharedTestInputs {
    blow: AtomicBool,
    valve1: AtomicBool,
    valve2: AtomicBool,
    valve3: AtomicBool,
    embouchure: AtomicU16,
    blowstrength: AtomicU16,
}

struct TestInputs {
    inputs: Arc<SharedTestInputs>,
}

impl Inputs for TestInputs {
    fn valve(&mut self, valve: Valve) -> bool {
        match valve {
            Valve::First => self.inputs.valve1.load(Ordering::Relaxed),
            Valve::Second => self.inputs.valve2.load(Ordering::Relaxed),
            Valve::Third => self.inputs.valve3.load(Ordering::Relaxed),
        }
    }

    fn blow(&mut self) -> bool {
        self.inputs.blow.load(Ordering::Relaxed)
    }

    fn embouchure(&mut self) -> Embouchure {
        Embouchure::from_bits(self.inputs.embouchure.load(Ordering::Relaxed))
    }

    fn blowstrength(&mut self) -> trumpet_synth::trumpet::BlowStrength {
        BlowStrength::from_bits(self.inputs.blowstrength.load(Ordering::Relaxed))
    }
}

#[derive(Debug)]
pub enum TesterInput {
    NoInput { samples: u32 },
    Blow(bool),
    Valve { valve: Valve, state: bool },
    Embouchure(u16),
    Blowstrength(u16),
}

pub struct TrumpetSynthTester {
    synthesizer: trumpet_synth::synth::TrumpetSynth,
    fifo: Arc<Mutex<Vec<u32>>>,
    inputs: Arc<SharedTestInputs>,
    interface: TrumpetInterface<TestFifo, TestInputs>,
    tester_input: Vec<TesterInput>,
}

impl TrumpetSynthTester {
    pub fn new(tester_input: Vec<TesterInput>) -> Self {
        let fifo = Arc::new(Mutex::new(Vec::new()));
        let inputs = Arc::new(SharedTestInputs {
            blow: AtomicBool::new(false),
            valve1: AtomicBool::new(false),
            valve2: AtomicBool::new(false),
            valve3: AtomicBool::new(false),
            embouchure: AtomicU16::new(0),
            blowstrength: AtomicU16::new(2048),
        });
        let interface = TrumpetInterface::new(IO {
            fifo: TestFifo {
                fifo: Arc::clone(&fifo),
            },
            inputs: TestInputs {
                inputs: Arc::clone(&inputs),
            },
        });

        Self {
            synthesizer: trumpet_synth::synth::create(),
            fifo,
            interface,
            tester_input,
            inputs: inputs,
        }
    }

    pub fn run(&mut self) -> Vec<i16> {
        let mut result = Vec::new();

        while let Some(inp) = self.tester_input.pop() {
            dbg!(&inp);
            result.extend(self.handle_input(inp));

            self.interface.run();
            let mut queue = self.fifo.lock().unwrap();

            while let Some(command_as_u32) = queue.pop() {
                let command = Command::deserialize(command_as_u32).expect("Invalid command");
                self.synthesizer.run_command(command);
            }
        }

        result
    }

    pub fn handle_input(&mut self, input: TesterInput) -> Vec<i16> {
        let mut result = Vec::new();

        match input {
            TesterInput::NoInput { samples } => {
                for _ in 0..samples {
                    result.push(self.synthesizer.next().to_bits())
                }
            }
            TesterInput::Blow(state) => self.inputs.blow.store(state, Ordering::Relaxed),
            TesterInput::Valve { valve, state } => match valve {
                Valve::First => self.inputs.valve1.store(state, Ordering::Relaxed),
                Valve::Second => self.inputs.valve2.store(state, Ordering::Relaxed),
                Valve::Third => self.inputs.valve3.store(state, Ordering::Relaxed),
            },
            TesterInput::Embouchure(value) => {
                self.inputs.embouchure.store(value, Ordering::Relaxed)
            }
            TesterInput::Blowstrength(value) => {
                self.inputs.blowstrength.store(value, Ordering::Relaxed)
            }
        }

        result
    }
}

#[test]
fn test_trumpet_frequency() {
    let mut tester = TrumpetSynthTester::new(vec![
        TesterInput::Blow(true),
        TesterInput::NoInput { samples: 33 },
        TesterInput::Blow(false),
        TesterInput::NoInput { samples: 11 },
        TesterInput::Valve {
            valve: Valve::First,
            state: true,
        },
        TesterInput::Blow(true),
        TesterInput::NoInput { samples: 33 },
        TesterInput::Blow(false),
        TesterInput::NoInput { samples: 11 },
    ]);

    let result = tester.run();

    println!("{:?}", result); // TODO: broken, gives only zeroes

    // TODO: resolve todo's in test impls of hardware, make mutexes for state?
    // TODO: make sure the inputs change between calls to run and observe behaviour
}
