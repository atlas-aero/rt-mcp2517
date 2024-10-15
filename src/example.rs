//! # Mock dummy structure for doc examples
//!
//! This code can be removed by disabling the `example` feature
use alloc::vec;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::convert::Infallible;
use embedded_hal::blocking::spi::Transfer;
use embedded_hal::digital::v2::OutputPin;
use embedded_time::clock::Error;
use embedded_time::duration::{Duration, Fraction};
use embedded_time::fixed_point::FixedPoint;
use embedded_time::timer::param::{Armed, OneShot};
use embedded_time::{Clock, Instant, Timer};

#[derive(Default)]
pub struct ExampleSPIBus {
    read_calls: u32,
}

impl Transfer<u8> for ExampleSPIBus {
    type Error = Infallible;

    fn transfer<'w>(&mut self, words: &'w mut [u8]) -> Result<&'w [u8], Self::Error> {
        // write command -> returns empty buffer
        if (words[0] >> 4) == 0x2 {
            return Ok(&[0u8; 3]);
        }

        // RAM read command
        if words.len() == 8 && words == [0u8; 8] {
            words.iter_mut().enumerate().for_each(|(i, val)| {
                *val += (i + 1) as u8;
            });
            return Ok(&[0u8; 8]);
        }

        // SFR Read command
        if words[0] >= 0x3 {
            return match words[1] {
                // addr: C1CON reg 2
                0x2 => {
                    // configuration mode
                    if self.read_calls == 0 {
                        self.read_calls += 1;
                        return Ok(&[0, 0, 0b1001_0100]);
                    }

                    // return operation mode NormalCANFD mode (called in configure and during transmission)
                    Ok(&[0x0, 0x0, 0b0000_0000])
                }
                // C1FIFOSTA2
                0x6C => Ok(&[0, 0, 0x1]),
                // C1FIFOUA2 (2 extra bytes in beginning for cmd+addr)
                0x70 => Ok(&[0, 0, 0, 0, 0x04, 0xA2]),
                // C1FIFOCON2 register 1
                0x69 => Ok(&[0, 0, 0]),
                // C1FIFOSTA1
                0x60 => Ok(&[0, 0, 0x1]),
                // C1FIFOUA1
                0x64 => Ok(&[0, 0, 0, 0x04, 0x7C]),

                _ => Ok(&[0, 0, 0]),
            };
        }

        Ok(&[0u8; 3])
    }
}

pub struct ExampleCSPin {}

impl OutputPin for ExampleCSPin {
    type Error = Infallible;

    fn set_low(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
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
