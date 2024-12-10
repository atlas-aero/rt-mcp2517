use alloc::vec::Vec;
use core::cell::RefCell;
use core::fmt::{Debug, Formatter};
use embedded_hal::spi::{Error, ErrorType, Operation};
use embedded_hal::spi::{ErrorKind, SpiDevice};
use embedded_time::clock::Error as ClockError;
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

    fn try_now(&self) -> Result<Instant<Self>, ClockError> {
        if self.next_instants.borrow().len() == 0 {
            return Err(ClockError::Unspecified);
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

pub struct MockDeviceBuilder {
    device: MockSPIDevice,
}

#[derive(Debug, Clone)]
pub enum SPIError {
    Error1,
}

impl Error for SPIError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Other
    }
}

mock! {
    pub SPIDevice {}

    impl ErrorType for SPIDevice {
        type Error = SPIError;
    }
    impl SpiDevice<u8> for SPIDevice {

        fn transaction<'a>(
        &mut self,
        operations: &mut [Operation<'a, u8>]
        ) -> Result<(), SPIError>;
    }

    impl PartialEq for SPIDevice {
        fn eq(&self, _other: &Self) -> bool {
            true
        }
    }
    impl Debug for SPIDevice {
    fn fmt<'a>(&self, f: &mut Formatter<'a>) -> core::fmt::Result {
            f.debug_struct("MockSpiDevice").finish()
        }
    }
}
