use crate::can::BusError::{CSError, TransferError};
use crate::can::ConfigError::{ClockError, ConfigurationModeTimeout, RequestModeTimeout};
use crate::config::{ClockConfiguration, Configuration};
use crate::status::{OperationMode, OperationStatus, OscillatorStatus};
use core::marker::PhantomData;
use embedded_hal::blocking::spi::Transfer;
use embedded_hal::digital::v2::OutputPin;
use embedded_time::duration::Milliseconds;
use embedded_time::Clock;
use log::debug;

const REGISTER_C1CON: u16 = 0x000;
const REGISTER_OSC: u16 = 0xE00;

/// FIFO index for receiving CAN messages
const FIFO_RX_INDEX: u8 = 1;

/// FIFO index for transmitting CAN messages
const FIFO_TX_INDEX: u8 = 2;

/// General SPI Errors
#[derive(Debug, PartialEq)]
pub enum BusError<B, CS> {
    /// Failed setting state of CS pin
    CSError(CS),

    /// SPI transfer failed
    TransferError(B),
}

/// Configuration errors
#[derive(Debug, PartialEq)]
pub enum ConfigError<B, CS> {
    /// Low level bus communication error
    BusError(BusError<B, CS>),

    /// Internal clock error
    ClockError,

    /// No configuration mode within timeout of 2 ms
    ConfigurationModeTimeout,

    /// Device did not enter given request mode within timeout of 2 ms
    RequestModeTimeout,
}

/// Main MCP2517 CAN controller device
pub struct Controller<B: Transfer<u8>, CS: OutputPin, CLK: Clock> {
    /// SPI bus
    bus: B,

    /// CS pin
    pin_cs: CS,

    /// System clock
    clock: PhantomData<CLK>,
}

impl<B: Transfer<u8>, CS: OutputPin, CLK: Clock> Controller<B, CS, CLK> {
    pub fn new(bus: B, pin_cs: CS) -> Self {
        Self {
            bus,
            pin_cs,
            clock: Default::default(),
        }
    }

    /// Configures the controller with the given settings
    pub fn configure(&mut self, config: &Configuration, clock: &CLK) -> Result<(), ConfigError<B::Error, CS::Error>> {
        self.enable_mode(OperationMode::Configuration, clock, ConfigurationModeTimeout)?;
        self.write_register(REGISTER_OSC, config.clock.as_register())?;

        self.write_register(Self::fifo_register(FIFO_RX_INDEX) + 3, config.fifo.as_rx_register())?;
        self.write_register(Self::fifo_register(FIFO_TX_INDEX) + 2, config.fifo.as_tx_register_2())?;
        self.write_register(Self::fifo_register(FIFO_TX_INDEX) + 3, config.fifo.as_tx_register_3())?;
        self.write_register(Self::fifo_register(FIFO_TX_INDEX), config.fifo.as_tx_register_0())?;

        self.enable_mode(config.mode.to_operation_mode(), clock, RequestModeTimeout)?;
        Ok(())
    }

    /// Reads and returns the operation status
    pub fn read_operation_status(&mut self) -> Result<OperationStatus, BusError<B::Error, CS::Error>> {
        let data = self.read_register(REGISTER_C1CON + 2)?;
        Ok(OperationStatus::from_register(data))
    }

    /// Reads and returns the oscillator status
    pub fn read_oscillator_status(&mut self) -> Result<OscillatorStatus, BusError<B::Error, CS::Error>> {
        let data = self.read_register(REGISTER_OSC + 1)?;
        Ok(OscillatorStatus::from_register(data))
    }

    /// Reads and returns the current clock configuration
    pub fn read_clock_configuration(&mut self) -> Result<ClockConfiguration, BusError<B::Error, CS::Error>> {
        let data = self.read_register(REGISTER_OSC)?;
        Ok(ClockConfiguration::from_register(data))
    }

    /// Enters the given mode, aborts all running transactions
    /// and waits max. 2 ms for the given mode to be reached
    fn enable_mode(
        &mut self,
        mode: OperationMode,
        clock: &CLK,
        timeout_error: ConfigError<B::Error, CS::Error>,
    ) -> Result<(), ConfigError<B::Error, CS::Error>> {
        self.write_register(REGISTER_C1CON + 3, mode as u8 | (1 << 3))?;

        let target = clock.try_now()?.checked_add(Milliseconds::new(2)).ok_or(ClockError)?;
        let mut current_mode = None;

        while current_mode.is_none() || current_mode.unwrap() != mode {
            current_mode = Some(self.read_operation_status()?.mode);

            if clock.try_now()? > target {
                debug!("Device did not enter config mode within timeout. Current mode: {mode:?}");
                return Err(timeout_error);
            }
        }

        Ok(())
    }

    /// Writes the a single register byte
    fn write_register(&mut self, register: u16, value: u8) -> Result<(), BusError<B::Error, CS::Error>> {
        let mut buffer = self.cmd_buffer(register, Operation::Write);
        buffer[2] = value;

        self.transfer(&mut buffer)?;
        Ok(())
    }

    /// Reads a single register byte
    fn read_register(&mut self, register: u16) -> Result<u8, BusError<B::Error, CS::Error>> {
        let mut buffer = self.cmd_buffer(register, Operation::Read);
        self.transfer(&mut buffer)
    }

    /// Executes a SPI transfer with three bytes buffer and returns the last byte received
    fn transfer(&mut self, buffer: &mut [u8]) -> Result<u8, BusError<B::Error, CS::Error>> {
        self.pin_cs.set_low().map_err(CSError)?;
        let result = self.bus.transfer(buffer).map_err(TransferError);
        self.pin_cs.set_high().map_err(CSError)?;

        Ok(result?[2])
    }

    /// Creates a three byte command buffer for the given register
    fn cmd_buffer(&self, register: u16, operation: Operation) -> [u8; 3] {
        let mut buffer = [0x0u8; 3];
        let command = (register & 0x0FFF) | ((operation as u16) << 12);

        buffer[0] = (command >> 8) as u8;
        buffer[1] = (command & 0xFF) as u8;

        buffer
    }

    /// Returns the configuration register index for the given FIFO index
    fn fifo_register(fifo_index: u8) -> u16 {
        0x05C + 12 * (fifo_index as u16 - 1)
    }
}

/// Register operation type
#[derive(Copy, Clone)]
enum Operation {
    Write = 0b0010,
    Read = 0b0011,
}

impl<B, CS> From<embedded_time::clock::Error> for ConfigError<B, CS> {
    fn from(_error: embedded_time::clock::Error) -> Self {
        ClockError
    }
}

impl<B, CS> From<BusError<B, CS>> for ConfigError<B, CS> {
    fn from(value: BusError<B, CS>) -> Self {
        Self::BusError(value)
    }
}
