use embedded_hal::PwmPin;
use rp2040_hal::pwm::{self, Channel, FreeRunning, Pwm5, Pwm6, Slice};

pub struct TrumpetRgbLed {
    r_channel: Channel<Slice<Pwm5, FreeRunning>, pwm::A>,
    g_channel: Channel<Slice<Pwm5, FreeRunning>, pwm::B>,
    b_channel: Channel<Slice<Pwm6, FreeRunning>, pwm::A>,
}

impl TrumpetRgbLed {
    pub fn new(
        r_channel: Channel<Slice<Pwm5, FreeRunning>, pwm::A>,
        g_channel: Channel<Slice<Pwm5, FreeRunning>, pwm::B>,
        b_channel: Channel<Slice<Pwm6, FreeRunning>, pwm::A>,
    ) -> Self {
        Self {
            r_channel,
            g_channel,
            b_channel,
        }
    }

    pub fn color(&mut self, r: u8, g: u8, b: u8) {
        self.r_channel.set_duty((r as u16) << 8);
        self.g_channel.set_duty((g as u16) << 8);
        self.b_channel.set_duty((b as u16) << 8);
    }
}
