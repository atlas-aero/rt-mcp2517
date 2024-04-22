use crate::can::BusError::{CSError, TransferError};
use crate::can::ConfigError::{ClockError, ConfigurationModeTimeout, RequestModeTimeout};
use crate::config::{ClockConfiguration, Configuration};
use crate::message::{RxHeader, TxMessage};
use crate::registers::{FifoControlReg1, FifoStatusReg0, FilterMaskReg, FilterObjectReg};
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
#[derive(Debug, PartialEq)]
pub enum TestError<B, CS> {
    TestCSErr(CS),
    TestBErr(B),
}
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
#[derive(Debug, PartialEq)]
pub enum Error<B, CS> {
    ConfigErr(ConfigError<B, CS>),
    BusErr(BusError<B, CS>),
    InvalidPayloadLength(usize),
    InvalidRamAddress(u16),
}

impl<B, CS> From<BusError<B, CS>> for Error<B, CS> {
    fn from(value: BusError<B, CS>) -> Self {
        Error::BusErr(value)
    }
}

impl<B, CS> From<ConfigError<B, CS>> for Error<B, CS> {
    fn from(value: ConfigError<B, CS>) -> Self {
        Error::ConfigErr(value)
    }
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

        self.write_register(
            Self::fifo_control_register(FIFO_RX_INDEX) + 3,
            config.fifo.as_rx_register(),
        )?;
        self.write_register(
            Self::fifo_control_register(FIFO_TX_INDEX) + 2,
            config.fifo.as_tx_register_2(),
        )?;
        self.write_register(
            Self::fifo_control_register(FIFO_TX_INDEX) + 3,
            config.fifo.as_tx_register_3(),
        )?;
        self.write_register(
            Self::fifo_control_register(FIFO_TX_INDEX),
            config.fifo.as_tx_register_0(),
        )?;

        self.enable_filter(FIFO_RX_INDEX, 0)?;

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
    /// Enable filter for corresponding RX FIFO
    pub fn enable_filter(&mut self, fifo_index: u8, filter_index: u8) -> Result<(), BusError<B::Error, CS::Error>> {
        let filter_reg = Self::filter_control_register_byte(filter_index);
        // Filter must be disabled to modify FmBP
        self.disable_filter(filter_index)?;
        self.write_register(filter_reg, fifo_index)?;
        // Set FLTENm to enable filter
        self.write_register(filter_reg, (1 << 7) & fifo_index)?;
        Ok(())
    }

    pub fn disable_filter(&mut self, filter_index: u8) -> Result<(), BusError<B::Error, CS::Error>> {
        let filter_reg = Self::filter_control_register_byte(filter_index);
        self.write_register(filter_reg, !(1 << 7))?;
        Ok(())
    }
    /// Set corresponding filter object
    pub fn set_filter_object(
        &mut self,
        filter_index: u8,
        filter: FilterObjectReg,
    ) -> Result<(), BusError<B::Error, CS::Error>> {
        let filter_object_reg = Self::filter_object_register(filter_index);
        self.disable_filter(filter_index)?;
        let filter_value = u32::from(filter);
        self.write32(filter_object_reg, filter_value)?;
        Ok(())
    }
    /// Set corresponding filter mask
    pub fn set_filter_mask(
        &mut self,
        filter_index: u8,
        filter_mask: FilterMaskReg,
    ) -> Result<(), BusError<B::Error, CS::Error>> {
        let filter_mask_reg = Self::filter_mask_register(filter_index);
        let mask_value = u32::from(filter_mask);
        self.write32(filter_mask_reg, mask_value)?;
        Ok(())
    }
    /// Writes a single register byte
    fn write_register(&mut self, register: u16, value: u8) -> Result<(), BusError<B::Error, CS::Error>> {
        let mut buffer = self.cmd_buffer(register, Operation::Write);
        buffer[2] = value;

        self.transfer(&mut buffer)?;
        Ok(())
    }

    fn write32(&mut self, register: u16, value: u32) -> Result<(), BusError<B::Error, CS::Error>> {
        let mut buffer = [0u8; 2];
        let command = (register & 0x0FFF) | ((Operation::Write as u16) << 12);

        buffer[0] = (command >> 8) as u8;
        buffer[1] = (command & 0xFF) as u8;
        let mut value_bytes = value.to_le_bytes();

        self.pin_cs.set_low().map_err(CSError)?;
        self.bus.transfer(&mut buffer).map_err(TransferError)?;
        self.bus.transfer(&mut value_bytes).map_err(TransferError)?;
        self.pin_cs.set_high().map_err(CSError)?;

        Ok(())
    }

    pub fn reset(&mut self) -> Result<(), BusError<B::Error, CS::Error>> {
        let mut buffer = self.cmd_buffer(0u16, Operation::Reset);
        self.transfer(&mut buffer)?;
        Ok(())
    }

    pub fn transmit(&mut self, message: TxMessage) -> Result<(), Error<B::Error, CS::Error>> {
        // make sure there is space for new message in TX FIFO
        // read byte 0 of TX FIFO status register

        let mut txfifo_status_byte0 = self.read_register(Self::fifo_status_register(FIFO_TX_INDEX))?;
        let mut txfifo_status_reg0 = FifoStatusReg0::from(txfifo_status_byte0);

        // block until there is room available for new message in TX FIFO
        while !txfifo_status_reg0.tfnrfnif() {
            txfifo_status_byte0 = self.read_register(Self::fifo_status_register(FIFO_TX_INDEX))?;
            txfifo_status_reg0 = FifoStatusReg0::from(txfifo_status_byte0);
        }

        // make sure length of payload is consistent with CAN operation mode
        let operation_status = self.read_operation_status()?;

        if message.length > 8 && operation_status.mode != OperationMode::NormalCANFD {
            return Err(Error::InvalidPayloadLength(message.length));
        }

        // get address in which to write next message in TX FIFO (should not be read in configuration mode)
        let address = self.read32(Self::fifo_user_address_register(FIFO_TX_INDEX))?;

        // load message in TX FIFO
        self.write_fifo(address as u16, message)?;
        // Request transmission (set txreq) and set uinc in TX FIFO control register byte 1
        self.write_register(Self::fifo_control_register(FIFO_TX_INDEX) + 1, 0x03)?;

        // read TX FIFO control register byte 1

        let mut txfifo_control_byte1 = self.read_register(Self::fifo_control_register(FIFO_TX_INDEX) + 1)?;
        let mut txfifo_control_reg = FifoControlReg1::from(txfifo_control_byte1);

        // block till txreq is cleared confirming that all messages in TX FIFO are transmitted
        while txfifo_control_reg.txreq() {
            txfifo_control_byte1 = self.read_register(Self::fifo_control_register(FIFO_TX_INDEX) + 1)?;
            txfifo_control_reg = FifoControlReg1::from(txfifo_control_byte1);
        }
        Ok(())
    }

    pub fn receive(&mut self, data: &mut [u8]) -> Result<RxHeader, Error<B::Error, CS::Error>> {
        let rxfifo_status_byte0 = self.read_register(Self::fifo_status_register(FIFO_RX_INDEX))?;
        let rxfifo_status_reg0 = FifoStatusReg0::from(rxfifo_status_byte0);

        // block until fifo contains at least one message
        while !rxfifo_status_reg0.tfnrfnif() {}

        let address = self.read32(Self::fifo_user_address_register(FIFO_RX_INDEX))?;
        // read message object
        self.read_fifo(address as u16, data)?;

        // set uinc
        self.write_register(Self::fifo_control_register(FIFO_RX_INDEX) + 1, 1)?;

        let mut header_bytes = [0u8; 8];
        header_bytes.copy_from_slice(&data[..8]);
        let message_header = RxHeader::from_bytes(header_bytes);

        Ok(message_header)
    }

    /// Insert message object in TX FIFO
    fn write_fifo(&mut self, register: u16, mut message: TxMessage) -> Result<(), Error<B::Error, CS::Error>> {
        self.verify_ram_address(register, message.length)?;

        let mut buffer = [0u8; 10];
        let command = (register & 0x0FFF) | ((Operation::Write as u16) << 12);

        buffer[0] = (command >> 8) as u8;
        buffer[1] = (command & 0xFF) as u8;
        buffer[2..].copy_from_slice(&message.header.into_bytes());

        self.pin_cs.set_low().map_err(CSError)?;
        self.bus.transfer(&mut buffer).map_err(TransferError)?;
        //      self.bus
        //          .transfer(&mut message.payload[..message.length])
        //          .map_err(TransferError)?;
        self.bus.transfer(&mut message.buff).map_err(TransferError)?;
        self.pin_cs.set_high().map_err(CSError)?;
        Ok(())
    }
    /// Read message from RX FIFO
    fn read_fifo(&mut self, register: u16, data: &mut [u8]) -> Result<(), Error<B::Error, CS::Error>> {
        let mut buffer = [0u8; 2];
        let command = (register & 0x0FFF) | ((Operation::Read as u16) << 12);

        buffer[0] = (command >> 8) as u8;
        buffer[1] = (command & 0xFF) as u8;

        self.pin_cs.set_low().map_err(CSError)?;
        self.bus.transfer(&mut buffer).map_err(TransferError)?;
        self.bus.transfer(data).map_err(TransferError)?;
        self.pin_cs.set_high().map_err(CSError)?;
        Ok(())
    }

    /// 4-byte register read
    fn read32(&mut self, register: u16) -> Result<u32, BusError<B::Error, CS::Error>> {
        // create 6 byte cmd buffer (2 bytes cmd+addr , 4 bytes for register value)
        let mut buffer = [0u8; 6];
        let command = (register & 0x0FFF) | ((Operation::Read as u16) << 12);

        buffer[0] = (command >> 8) as u8;
        buffer[1] = (command & 0xFF) as u8;

        self.pin_cs.set_low().map_err(CSError)?;
        let result = self.bus.transfer(&mut buffer).map_err(TransferError)?;
        self.pin_cs.set_high().map_err(CSError)?;

        let mut data_read = [0u8; 4];
        data_read.clone_from_slice(&result[2..]);

        // reverse so that msb byte of register is at the first index
        let result = u32::from_le_bytes(data_read);
        Ok(result)
    }
    /// Verify address within RAM bounds
    fn verify_ram_address(&self, addr: u16, data_length: usize) -> Result<(), Error<B::Error, CS::Error>> {
        if addr < 0x400 || (addr + (data_length as u16)) > 0xBFF {
            return Err(Error::InvalidRamAddress(addr));
        }
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

    /// Returns the configuration register address for the given FIFO index
    fn fifo_control_register(fifo_index: u8) -> u16 {
        0x05C + 12 * (fifo_index as u16 - 1)
    }

    /// Returns the status register address for the given FIFO index
    fn fifo_status_register(fifo_index: u8) -> u16 {
        0x60 + 12 * (fifo_index as u16 - 1)
    }

    /// Returns the address of fifo user address register for the given index
    fn fifo_user_address_register(fifo_index: u8) -> u16 {
        0x64 + 12 * (fifo_index as u16 - 1)
    }
    /// returns the filter control register address byte of the corresponding filter
    fn filter_control_register_byte(filter_index: u8) -> u16 {
        0x1D0 + filter_index as u16
    }
    /// returns the filter object register address of corresponding filter
    fn filter_object_register(filter_index: u8) -> u16 {
        0x1F0 + 8 * (filter_index as u16)
    }
    /// returns the filter mask register address of corresponding filter
    fn filter_mask_register(filter_index: u8) -> u16 {
        0x1F4 + 8 * (filter_index as u16)
    }
}

/// Register operation type
#[derive(Copy, Clone)]
enum Operation {
    Reset = 0b0000,
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
