//! # Mock dummy structure for doc examples
//!
//! This code can be removed by disabling the `example` feature
use alloc::vec;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::convert::Infallible;
use embedded_hal::spi::{ErrorType, Operation, SpiDevice};
use embedded_time::clock::Error;
use embedded_time::duration::{Duration, Fraction};
use embedded_time::fixed_point::FixedPoint;
use embedded_time::timer::param::{Armed, OneShot};
use embedded_time::{Clock, Instant, Timer};

#[derive(Default, Debug)]
pub struct ExampleSPIDevice {
    read_calls: u32,
}

impl ErrorType for ExampleSPIDevice {
    type Error = Infallible;
}

impl SpiDevice<u8> for ExampleSPIDevice {
    fn transaction(&mut self, operations: &mut [Operation<'_, u8>]) -> Result<(), Self::Error> {
        if operations[0] == Operation::Write(&[0x30, 0x70]) {
            // C1FIFOUA2
            if let Operation::Read(read) = &mut operations[1] {
                read.copy_from_slice(&[0, 0, 0x04, 0xA2]);
                return Ok(());
            }
        }

        if operations[0] == Operation::Write(&[0x30, 0x64]) {
            // C1FIFOUA1
            if let Operation::Read(read) = &mut operations[1] {
                read.copy_from_slice(&[0, 0, 0x04, 0x7C]);
                return Ok(());
            }
        }

        // RAM Read command
        if let Operation::Write(_) = operations[0] {
            if operations.len() == 2 {
                if let Operation::Read(read) = &mut operations[1] {
                    if read.len() == 8 {
                        read.iter_mut().enumerate().for_each(|(i, val)| {
                            *val += (i + 1) as u8;
                        });
                        return Ok(());
                    }
                }
            }
        }

        Ok(())
    }

    fn transfer_in_place(&mut self, buf: &mut [u8]) -> Result<(), Self::Error> {
        if (buf[0] >> 4) == 0x2 {
            return Ok(());
        }

        // SFR Read command
        if buf[0] == 0x30 {
            match buf[1] {
                // addr: C1CON reg 2
                0x2 => {
                    // configuration mode
                    if self.read_calls == 0 {
                        self.read_calls += 1;
                        buf.copy_from_slice(&[0, 0, 0b1001_0100]);
                        return Ok(());
                    }

                    // return operation mode NormalCANFD mode (called in configure and during transmission)
                    buf.copy_from_slice(&[0x0, 0x0, 0b0000_0000]);
                }
                // C1FIFOSTA2
                0x6C => buf.copy_from_slice(&[0, 0, 0x1]),
                // C1FIFOCON2 register 1
                0x69 => buf.copy_from_slice(&[0, 0, 0]),
                // C1FIFOSTA1
                0x60 => buf.copy_from_slice(&[0, 0, 0x1]),
                _ => {}
            }
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ExampleClock {
    pub next_instants: RefCell<Vec<u64>>,
}

impl ExampleClock {
    pub fn new(next_instants: Vec<u64>) -> Self {
        Self {
            next_instants: RefCell::new(next_instants),
        }
    }
}

impl Default for ExampleClock {
    fn default() -> Self {
        Self::new(vec![
            100,    // Config mode: Timer start,
            200,    // Config mode: First expiration check
            10_000, // Request mode: Timer start
            10_100, // Request mode: First expiration check
        ])
    }
}

impl Clock for ExampleClock {
    type T = u64;
    const SCALING_FACTOR: Fraction = Fraction::new(1, 1_000_000);

    fn try_now(&self) -> Result<Instant<Self>, Error> {
        if self.next_instants.borrow().len() == 0 {
            return Err(Error::Unspecified);
        }

        Ok(Instant::new(self.next_instants.borrow_mut().remove(0)))
    }

    fn new_timer<Dur: Duration + FixedPoint>(&self, duration: Dur) -> Timer<OneShot, Armed, Self, Dur> {
        Timer::new(self, duration)
    }
}
