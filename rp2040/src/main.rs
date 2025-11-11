#![no_std]
#![no_main]

pub mod io;
pub mod rgb;

#[link_section = ".boot2"]
#[no_mangle]
#[used]
pub static BOOT2_FIRMWARE: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

use cortex_m::singleton;
#[allow(unused_imports)]
#[cfg(feature = "defmt")]
use defmt::{error, info, warn};
use defmt_rtt as _;
use fugit::HertzU32;
use io::{Rp2040Inputs, SioFifo};
#[allow(unused_imports)]
#[cfg(not(feature = "defmt"))]
use log::{error, info, warn};
use panic_probe as _;
use pio_proc::pio_file;
use rgb::TrumpetRgbLed;
use rp2040_hal::gpio::bank0::Gpio0;
use rp2040_hal::gpio::{DynFunction, DynPinId, FunctionSioInput, Pin, PullDown};
use rp2040_hal::pac;
use rp2040_hal::{
    adc::AdcPin,
    clocks::{Clock, ClockSource, ClocksManager, InitError},
    dma::{double_buffer, DMAExt},
    entry,
    gpio::{
        self,
        bank0::{Gpio10, Gpio11, Gpio12},
        FunctionPio0, FunctionPwm, PullUp,
    },
    multicore::{Multicore, Stack},
    pio::{Buffers, PIOBuilder, PIOExt, PinDir, ShiftDirection},
    pll::{common_configs::PLL_USB_48MHZ, setup_pll_blocking, PLLConfig},
    pwm,
    sio::Sio,
    watchdog::Watchdog,
    xosc::setup_xosc_blocking,
    Adc,
};

use common::consts::*;
use rytmos_synth::{commands::Command, synth::Synth};
use trumpet_synth::{interface::TrumpetInterface, io::IO};

static mut CORE1_STACK: Stack<4096> = Stack::new();

#[allow(dead_code)]
fn setup_dual_adc_and_dac(sys_freq: HertzU32) -> ! {
    let mut pac = unsafe { pac::Peripherals::steal() };
    let mut sio = Sio::new(pac.SIO);
    let pins = gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let (mut pio0, sm0, _, _, sm3) = pac.PIO0.split(&mut pac.RESETS);

    // -- move to lib --

    let sck_pin: gpio::Pin<gpio::bank0::Gpio9, gpio::FunctionPio0, gpio::PullDown> =
        pins.gpio9.reconfigure();
    {
        #[rustfmt::skip]
        let sck_pio_program = pio_proc::pio_asm!(
            ".wrap_target",
            "    set pins, 0b1",
            "    set pins, 0b0",
            ".wrap",
        );

        let installed = pio0.install(&sck_pio_program.program).unwrap();

        let sck_clock_divisor: u16 = 6; // 256 times the lr clock => 48k => 12.288MHz => lib found clock / 2

        let (mut sck_sm, _, _) = rp2040_hal::pio::PIOBuilder::from_installed_program(installed)
            .set_pins(sck_pin.id().num, 1)
            .clock_divisor_fixed_point(sck_clock_divisor, 64)
            .build(sm3);

        sck_sm.set_pindirs([(sck_pin.id().num, PinDir::Output)]);
        sck_sm.start();
        info!("sck state machine started");
    }

    // -- ^^^^^^^^^^^ --

    let dac_output = rp2040_i2s::I2SOutput::new(
        &mut pio0,
        rp2040_i2s::PioClockDivider::FromSystemClock(sys_freq),
        sm0,
        pins.gpio6,
        pins.gpio7,
        pins.gpio8,
    )
    .unwrap();

    let (dac_sm, _, dac_fifo_tx) = dac_output.split();

    dac_sm.start();

    let dma_channels = pac.DMA.split(&mut pac.RESETS);

    let i2s_tx_buf1 = singleton!(: [u32; BUFFER_SIZE*2] = [0; BUFFER_SIZE*2]).unwrap();
    let i2s_tx_buf2 = singleton!(: [u32; BUFFER_SIZE*2] = [0; BUFFER_SIZE*2]).unwrap();
    let i2s_dma_config = double_buffer::Config::new(
        (dma_channels.ch0, dma_channels.ch1),
        i2s_tx_buf1,
        dac_fifo_tx,
    );
    let i2s_tx_transfer = i2s_dma_config.start();
    let mut i2s_tx_transfer = i2s_tx_transfer.read_next(i2s_tx_buf2);

    // make synth
    let mut synth = trumpet_synth::synth::create();
    let mut sample = 0i16;
    let mut warned = false;

    loop {
        sio.fifo
            .read()
            .and_then(Command::deserialize)
            .inspect(|&command| synth.run_command(command));

        if !warned && i2s_tx_transfer.is_done() {
            warn!("i2s transfer already done, probably late.");
            warned = true;
        }

        let (next_tx_buf, next_tx_transfer) = i2s_tx_transfer.wait();
        for (i, e) in next_tx_buf.iter_mut().enumerate() {
            if i % 2 == 0 {
                sample = synth.next().to_bits();
                *e = (sample as u32) >> 4;
            } else {
                *e = (sample as u32) >> 4;
            }
            *e <<= 16;
        }

        i2s_tx_transfer = next_tx_transfer.read_next(next_tx_buf);
    }
}

fn synth_core(sys_freq: u32) -> ! {
    let mut pac = unsafe { pac::Peripherals::steal() };
    let core = unsafe { pac::CorePeripherals::steal() };
    let mut sio = Sio::new(pac.SIO);
    let pins = gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );
    let mut delay = cortex_m::delay::Delay::new(core.SYST, sys_freq);

    let i2s_sck_pin = pins.gpio9.into_function::<FunctionPio0>();
    let i2s_din_pin = pins.gpio6.into_function::<FunctionPio0>();
    let i2s_bck_pin = pins.gpio7.into_function::<FunctionPio0>();
    let i2s_lck_pin = pins.gpio8.into_function::<FunctionPio0>();

    let pio_i2s_mclk_output = pio_file!("src/i2s.pio", select_program("mclk_output")).program;
    let pio_i2s_send_master = pio_file!("src/i2s.pio", select_program("i2s_out_master")).program;

    let (mut pio, sm0, sm1, _, _) = pac.PIO0.split(&mut pac.RESETS);
    let pio_i2s_mclk_output = pio.install(&pio_i2s_mclk_output).unwrap();
    let pio_i2s_send_master = pio.install(&pio_i2s_send_master).unwrap();

    let (mut sm0, _rx0, _tx0) = PIOBuilder::from_installed_program(pio_i2s_mclk_output)
        .set_pins(i2s_sck_pin.id().num, 1)
        .clock_divisor_fixed_point(MCLK_CLOCKDIV_INT, MCLK_CLOCKDIV_FRAC)
        .build(sm0);

    let (mut sm1, _rx1, tx1) = PIOBuilder::from_installed_program(pio_i2s_send_master)
        .out_pins(i2s_din_pin.id().num, 1)
        .side_set_pin_base(i2s_bck_pin.id().num)
        .clock_divisor_fixed_point(I2S_PIO_CLOCKDIV_INT, I2S_PIO_CLOCKDIV_FRAC)
        .out_shift_direction(ShiftDirection::Left)
        .autopull(true)
        .pull_threshold(16u8)
        .buffers(Buffers::OnlyTx)
        .build(sm1);

    sm0.set_pindirs([(i2s_sck_pin.id().num, PinDir::Output)]);
    sm0.start();
    sm1.set_pindirs([
        (i2s_din_pin.id().num, PinDir::Output),
        (i2s_lck_pin.id().num, PinDir::Output),
        (i2s_bck_pin.id().num, PinDir::Output),
    ]);
    sm1.start();

    let dma_channels = pac.DMA.split(&mut pac.RESETS);
    let i2s_tx_buf1 = singleton!(: [u32; BUFFER_SIZE*2] = [0; BUFFER_SIZE*2]).unwrap();
    let i2s_tx_buf2 = singleton!(: [u32; BUFFER_SIZE*2] = [0; BUFFER_SIZE*2]).unwrap();
    let i2s_dma_config =
        double_buffer::Config::new((dma_channels.ch0, dma_channels.ch1), i2s_tx_buf1, tx1);
    let i2s_tx_transfer = i2s_dma_config.start();
    let mut i2s_tx_transfer = i2s_tx_transfer.read_next(i2s_tx_buf2);

    delay.delay_ms(100);

    info!("Start Synth core.");

    let mut synth = trumpet_synth::synth::create();

    let mut sample = 0i16;

    let mut warned = false;

    loop {
        sio.fifo
            .read()
            .and_then(Command::deserialize)
            .inspect(|&command| synth.run_command(command));

        if !warned && i2s_tx_transfer.is_done() {
            warn!("i2s transfer already done, probably late.");
            warned = true;
        }

        let (next_tx_buf, next_tx_transfer) = i2s_tx_transfer.wait();
        for (i, e) in next_tx_buf.iter_mut().enumerate() {
            if i % 2 == 0 {
                sample = synth.next().to_bits();
                *e = (sample as u32) >> 4;
            } else {
                *e = (sample as u32) >> 4;
            }
            *e <<= 16;
        }

        i2s_tx_transfer = next_tx_transfer.read_next(next_tx_buf);
    }
}

pub const SYS_PLL_CONFIG_307P2MHZ: PLLConfig = PLLConfig {
    vco_freq: HertzU32::MHz(1536),
    refdiv: 1,
    post_div1: 5,
    post_div2: 1,
};

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let mut sio = Sio::new(pac.SIO);

    watchdog.enable_tick_generation((EXTERNAL_XTAL_FREQ_HZ.raw() / 1_000_000) as u8);

    let mut clocks = ClocksManager::new(pac.CLOCKS);

    let xosc = setup_xosc_blocking(pac.XOSC, EXTERNAL_XTAL_FREQ_HZ)
        .map_err(InitError::XoscErr)
        .ok()
        .unwrap();
    {
        let pll_sys = setup_pll_blocking(
            pac.PLL_SYS,
            xosc.operating_frequency(),
            SYS_PLL_CONFIG_307P2MHZ,
            &mut clocks,
            &mut pac.RESETS,
        )
        .map_err(InitError::PllError)
        .ok()
        .unwrap();
        let pll_usb = setup_pll_blocking(
            pac.PLL_USB,
            xosc.operating_frequency(),
            PLL_USB_48MHZ,
            &mut clocks,
            &mut pac.RESETS,
        )
        .map_err(InitError::PllError)
        .ok()
        .unwrap();
        clocks
            .reference_clock
            .configure_clock(&xosc, xosc.get_freq())
            .map_err(InitError::ClockError)
            .ok()
            .unwrap();
        clocks
            .system_clock
            .configure_clock(&pll_sys, pll_sys.get_freq())
            .map_err(InitError::ClockError)
            .ok()
            .unwrap();
        clocks
            .usb_clock
            .configure_clock(&pll_usb, pll_usb.get_freq())
            .map_err(InitError::ClockError)
            .ok()
            .unwrap();
        clocks
            .adc_clock
            .configure_clock(&pll_usb, pll_usb.get_freq())
            .map_err(InitError::ClockError)
            .ok()
            .unwrap();
        clocks
            .rtc_clock
            .configure_clock(&pll_usb, HertzU32::from_raw(46875u32))
            .map_err(InitError::ClockError)
            .ok()
            .unwrap();
        clocks
            .peripheral_clock
            .configure_clock(&clocks.system_clock, clocks.system_clock.freq())
            .map_err(InitError::ClockError)
            .ok()
            .unwrap();
    }

    let mut _delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    // Setup the other core
    let sys_freq = clocks.system_clock.freq().to_Hz();
    let mut mc = Multicore::new(&mut pac.PSM, &mut pac.PPB, &mut sio.fifo);
    let cores = mc.cores();
    let core1 = &mut cores[1];
    #[allow(static_mut_refs)]
    let _test = core1.spawn(unsafe { CORE1_STACK.take().unwrap() }, move || {
        synth_core(sys_freq)
    });

    let pins = rp2040_hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let valve_pins = [
        pins.gpio1.into_pull_up_input().into_dyn_pin(),
        pins.gpio16.into_pull_up_input().into_dyn_pin(), // Should be 2, is patched on the test PCB
        pins.gpio3.into_pull_up_input().into_dyn_pin(),
    ];

    let blow_pin: gpio::Pin<Gpio0, FunctionSioInput, PullUp> = pins.gpio0.reconfigure();

    let adc_pins: [AdcPin<Pin<DynPinId, DynFunction, PullDown>>; 2] = [
        AdcPin::new(pins.gpio26.reconfigure().into_dyn_pin()).unwrap(),
        AdcPin::new(pins.gpio27.reconfigure().into_dyn_pin()).unwrap(),
    ];

    let adc = Adc::new(pac.ADC, &mut pac.RESETS);

    let r: gpio::Pin<Gpio10, FunctionPwm, PullUp> = pins.gpio10.reconfigure();
    let g: gpio::Pin<Gpio11, FunctionPwm, PullUp> = pins.gpio11.reconfigure();
    let b: gpio::Pin<Gpio12, FunctionPwm, PullUp> = pins.gpio12.reconfigure();

    let pwm_slices = pwm::Slices::new(pac.PWM, &mut pac.RESETS);
    let mut pwm5 = pwm_slices.pwm5;
    pwm5.set_ph_correct();
    pwm5.enable();
    let mut pwm6 = pwm_slices.pwm6;
    pwm6.set_ph_correct();
    pwm6.enable();

    let mut r_channel = pwm5.channel_a;
    r_channel.output_to(r);
    let mut g_channel = pwm5.channel_b;
    g_channel.output_to(g);
    let mut b_channel = pwm6.channel_a;
    b_channel.output_to(b);

    let mut rgb = TrumpetRgbLed::new(r_channel, g_channel, b_channel);
    rgb.color(5, 0, 0);

    let io = IO {
        fifo: SioFifo(sio.fifo),
        inputs: Rp2040Inputs {
            valve_pins,
            blow_pin,
            adc,
            adc_pins,
        },
    };

    let mut interface = TrumpetInterface::new(io, 10);

    loop {
        interface.run();
        // let state = TrumpetInputState::read_from(&mut io.inputs);
        // defmt::info!(
        //     "{} {} {} {} {} {}",
        //     state.first,
        //     state.second,
        //     state.third,
        //     state.blow,
        //     state.embouchure.to_bits() >> 10,
        //     state.blowstrength.to_bits() >> 10
        // );
        // for _ in 0..100000 {
        //     asm::nop();
        // }
    }
}
