use alloc::vec::Vec;
use core::cell::RefCell;
use embedded_hal::blocking::spi::Transfer;
use embedded_hal::digital::v2::OutputPin;
use embedded_time::clock::Error;
use embedded_time::duration::Duration;
use embedded_time::fixed_point::FixedPoint;
use embedded_time::fraction::Fraction;
use embedded_time::timer::param::{Armed, OneShot};
use embedded_time::{Clock, Instant, Timer};
use mockall::mock;

#[derive(Debug, PartialEq, Eq)]
pub struct TestClock {
    pub next_instants: RefCell<Vec<u64>>,
}

impl TestClock {
    pub fn new(next_instants: Vec<u64>) -> Self {
        Self {
            next_instants: RefCell::new(next_instants),
        }
    }
}

impl Clock for TestClock {
    type T = u64;
    const SCALING_FACTOR: Fraction = Fraction::new(1, 1_000_000);

    fn try_now(&self) -> Result<Instant<Self>, Error> {
        if self.next_instants.borrow().len() == 0 {
            return Err(Error::Unspecified);
        }

        Ok(Instant::new(self.next_instants.borrow_mut().remove(0)))
    }

    fn new_timer<Dur>(&self, duration: Dur) -> Timer<OneShot, Armed, Self, Dur>
    where
        Dur: Duration + FixedPoint,
    {
        Timer::new(self, duration)
    }
}

mock! {
    pub SPIBus {}

    impl Transfer<u8> for SPIBus{
        type Error = u32;

        fn transfer<'w>(&mut self, words: &'w mut [u8]) -> Result<&'static [u8], u32>;
    }
}

mock! {
    pub Pin {}

    impl OutputPin for Pin {
        type Error = u32;

        fn set_low(&mut self) -> Result<(), u32>;
        fn set_high(&mut self) -> Result<(), u32>;
    }
}
