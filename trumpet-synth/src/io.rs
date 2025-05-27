pub struct IO<FIFO> {
    pub fifo: FIFO,
}

impl<FIFO> IO<FIFO>
where
    FIFO: Fifo,
{
    pub fn new(fifo: FIFO) -> Self {
        Self { fifo }
    }
}

pub trait Fifo {
    fn write(&mut self, value: u32);
}
