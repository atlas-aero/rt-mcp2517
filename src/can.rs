//!# CAN Controller device
//!
//!```
//!# use mcp2517::can::MCP2517;
//!# use mcp2517::config::Configuration;
//!# use mcp2517::example::*;
//!#
//! let sys_clk = ExampleClock::default();
//! let spi_dev = ExampleSPIDevice::default();
//!
//! // Initialize controller object
//! let mut can_controller = MCP2517::new(spi_dev);
//!
//! // Use default configuration settings
//! let can_config = Configuration::default();
//!
//! // Configure CAN controller
//! can_controller.configure(&can_config, &sys_clk).unwrap();
//! ```

use crate::config::{ClockConfiguration, Configuration};
use crate::filter::Filter;
use crate::message::{FrameType, MessageType, RxFrame, TxMessage};
use crate::registers::{FifoControlReg1, FifoStatusReg0, C1NBTCFG};
use crate::status::{OperationMode, OperationStatus, OscillatorStatus};
use byteorder::{BigEndian, ByteOrder, LittleEndian};
use core::fmt::Debug;
use core::marker::PhantomData;
use embedded_can::{ExtendedId, Id, StandardId};
use embedded_hal::spi::{Operation as SpiOperation, SpiDevice};
use embedded_time::duration::Milliseconds;
use embedded_time::Clock;
use log::debug;

const REGISTER_C1CON: u16 = 0x000;

const REGISTER_OSC: u16 = 0xE00;

const REGISTER_IOCON: u16 = 0xE04;

const IOCON_XSTBYEN: u8 = 1 << 6;

const REGISTER_C1NBTCFG: u16 = 0x004;

/// FIFO index for receiving CAN messages
const FIFO_RX_INDEX: u8 = 1;

/// FIFO index for transmitting CAN messages
const FIFO_TX_INDEX: u8 = 2;

#[derive(Debug)]
pub enum SpiError<D: SpiDevice<u8>> {
    BusError(D::Error),
}
impl<D: SpiDevice<u8>> PartialEq for SpiError<D> {
    fn eq(&self, other: &Self) -> bool {
        matches!((self, other), (Self::BusError(_), Self::BusError(_)))
    }
}

/// Possible CAN errors during Configuration/Transmission/Reception
#[derive(Debug, PartialEq)]
pub enum CanError<D: SpiDevice<u8>> {
    /// SPI bus transfer error
    BusErr(SpiError<D>),
    /// Internal clock error
    ClockError,
    /// No configuration mode within timeout of 2 ms
    ConfigurationModeTimeout,
    /// Device did not enter given request mode within timeout of 2 ms
    RequestModeTimeout,
    /// Invalid payload bytes length error
    InvalidPayloadLength(usize),
    /// Invalid Ram Address region error
    InvalidRamAddress(u16),
    /// Payload buffer length not a multiple of 4 bytes
    InvalidBufferSize(usize),
    /// Invalid or unsupported receive header
    InvalidFrameHeader,
    /// RX fifo empty error
    RxFifoEmptyErr,
    /// TX fifo buffer full error
    TxFifoFullErr,
}

impl<D: SpiDevice<u8>> From<SpiError<D>> for CanError<D> {
    fn from(value: SpiError<D>) -> Self {
        CanError::BusErr(value)
    }
}

/// Main MCP2517 CAN controller device
pub struct MCP2517<D: SpiDevice<u8>, CLK: Clock> {
    /// Device on SPI bus
    device: D,

    /// System clock
    clock: PhantomData<CLK>,
}

/// Trait for CAN controller
pub trait CanController {
    type Error;

    /// Transmit CAN message
    /// * `blocking`: if true, function blocks until TX fifo buffer is empty and till TXREQ bit is cleared
    fn transmit<const L: usize, T: MessageType<L>>(
        &mut self,
        message: &TxMessage<T, L>,
        blocking: bool,
    ) -> Result<(), Self::Error>;

    /// Receive a frame and return its ID, wire format, DLC and actual payload length.
    /// Only the received payload bytes are written; the buffer tail is unchanged.
    /// RX timestamps must be disabled (the default configuration).
    ///
    /// With `blocking = false`, an empty FIFO returns an error immediately. Invalid
    /// headers and frames too large for the buffer are consumed and return an error.
    fn receive(&mut self, data: &mut [u8], blocking: bool) -> Result<RxFrame, Self::Error>;

    /// Set corresponding filter and mask registers
    fn set_filter_object(&mut self, filter: Filter) -> Result<(), Self::Error>;
}

impl<D, CLK> CanController for MCP2517<D, CLK>
where
    D: SpiDevice<u8>,
    CLK: Clock,
{
    type Error = CanError<D>;

    fn transmit<const L: usize, T: MessageType<L>>(
        &mut self,
        message: &TxMessage<T, L>,
        blocking: bool,
    ) -> Result<(), Self::Error> {
        let fifo_status_reg = Self::fifo_status_register(FIFO_TX_INDEX);

        // Check if TX fifo is full
        while !self.fifo_tfnrfnif(fifo_status_reg)? {
            if !blocking {
                return Err(CanError::TxFifoFullErr);
            }
        }

        // make sure length of payload is consistent with CAN operation mode
        let operation_status = self.read_operation_status()?;

        if message.buff.len() > 8 && operation_status.mode != OperationMode::NormalCANFD {
            return Err(CanError::InvalidPayloadLength(message.buff.len()));
        }

        // get address in which to write next message in TX FIFO (should not be read in configuration mode)
        let user_address = self.read32(Self::fifo_user_address_register(FIFO_TX_INDEX))?;

        // calculate address of next Message Object according to
        // Equation 4-1 in MCP251XXFD Family Reference Manual
        if user_address > 0x7ff {
            return Err(CanError::InvalidRamAddress(user_address as u16));
        }
        let address = user_address + 0x400;

        // get address of TX FIFO control register byte 1
        let fifo_control_reg1 = Self::fifo_control_register(FIFO_TX_INDEX) + 1;

        // load message in TX FIFO
        self.write_fifo::<T, L>(address as u16, message)?;

        // Request transmission (set txreq) and set uinc in TX FIFO control register byte 1
        self.write_register(fifo_control_reg1, 0x03)?;

        // block till TXREQ is cleared confirming that all messages in TX FIFO are transmitted
        if blocking {
            while !self.txfifo_cleared(fifo_control_reg1)? {}
        }

        Ok(())
    }

    fn receive(&mut self, data: &mut [u8], blocking: bool) -> Result<RxFrame, Self::Error> {
        while !self.fifo_tfnrfnif(Self::fifo_status_register(FIFO_RX_INDEX))? {
            if !blocking {
                return Err(CanError::RxFifoEmptyErr);
            }
        }

        let offset = self.read32(Self::fifo_user_address_register(FIFO_RX_INDEX))?;
        // Check before arithmetic/casting so all-ones SPI reads cannot wrap into RAM.
        if offset > 0x7ff || offset % 4 != 0 {
            return Err(CanError::InvalidRamAddress(offset as u16));
        }

        let address = 0x400 + offset as u16;
        self.verify_ram_address(address, 8)?;
        let word0 = self.read32(address)?;
        let word1 = self.read32(address + 4)?;
        let fd = word1 & 0x80 != 0;
        let remote = word1 & 0x20 != 0;
        let brs = word1 & 0x40 != 0;

        // Ignore unimplemented bits (undefined on read). SID11 cannot be represented
        // by embedded_can::Id; RTR is invalid for FD and BRS is invalid for classic CAN.
        if word0 & (1 << 29) != 0 || (fd && remote) || (!fd && brs) {
            self.write_register(Self::fifo_control_register(FIFO_RX_INDEX) + 1, 1)?;
            return Err(CanError::InvalidFrameHeader);
        }

        let dlc = (word1 & 0xf) as u8;
        let frame_type = if fd {
            FrameType::Fd
        } else if remote {
            FrameType::Remote
        } else {
            FrameType::Data
        };
        let data_length = if remote {
            0
        } else if fd {
            [0, 1, 2, 3, 4, 5, 6, 7, 8, 12, 16, 20, 24, 32, 48, 64][dlc as usize]
        } else {
            usize::from(dlc.min(8))
        };

        self.verify_ram_address(address, 8 + data_length)?;
        if data_length > data.len() {
            self.write_register(Self::fifo_control_register(FIFO_RX_INDEX) + 1, 1)?;
            return Err(CanError::InvalidBufferSize(data.len()));
        }

        let sid = (word0 & 0x7ff) as u16;
        let id = if word1 & 0x10 != 0 {
            Id::Extended(ExtendedId::new(((sid as u32) << 18) | ((word0 >> 11) & 0x3ffff)).unwrap())
        } else {
            Id::Standard(StandardId::new(sid).unwrap())
        };

        if data_length != 0 {
            self.read_fifo(address, &mut data[..data_length])?;
        }
        self.write_register(Self::fifo_control_register(FIFO_RX_INDEX) + 1, 1)?;

        Ok(RxFrame {
            id,
            frame_type,
            dlc,
            data_length,
        })
    }

    /// Set corresponding filter and mask registers
    fn set_filter_object(&mut self, filter: Filter) -> Result<(), Self::Error> {
        let filter_object_reg = Self::filter_object_register(filter.index);
        let filter_mask_reg = Self::filter_mask_register(filter.index);

        self.disable_filter(filter.index)?;

        let filter_value = u32::from(filter.filter_bits);
        let mask_value = u32::from(filter.mask_bits);

        self.write32(filter_object_reg, filter_value)?;

        self.write32(filter_mask_reg, mask_value)?;

        let filter_control_reg = Self::filter_control_register_byte(filter.index);

        self.write_register(filter_control_reg, (1 << 7) | 1)?;

        Ok(())
    }
}

impl<D, CLK> MCP2517<D, CLK>
where
    D: SpiDevice,
    CLK: Clock,
{
    pub fn new(spi_dev: D) -> Self {
        Self {
            device: spi_dev,
            clock: Default::default(),
        }
    }

    /// Configures the controller with the given settings
    pub fn configure(&mut self, config: &Configuration, clock: &CLK) -> Result<(), CanError<D>> {
        self.enable_mode(OperationMode::Configuration, clock, CanError::ConfigurationModeTimeout)?;

        self.write_register(REGISTER_OSC, config.clock.as_register())?;

        let iocon = self.read_register(REGISTER_IOCON)?;
        let iocon = if config.xstby_enable {
            iocon | IOCON_XSTBYEN
        } else {
            iocon & !IOCON_XSTBYEN
        };
        self.write_register(REGISTER_IOCON, iocon)?;

        let nbr_values = config.bit_rate.calculate_values();
        let nbr_reg = C1NBTCFG::from_bytes(nbr_values).into();

        self.write32(REGISTER_C1NBTCFG, nbr_reg)?;

        self.write_register(
            Self::fifo_control_register(FIFO_RX_INDEX) + 3,
            config.fifo.as_rx_register_3(),
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

        self.enable_mode(config.mode.to_operation_mode(), clock, CanError::RequestModeTimeout)?;

        Ok(())
    }

    /// Disable corresponding filter
    pub fn disable_filter(&mut self, filter_index: u8) -> Result<(), CanError<D>> {
        let filter_reg = Self::filter_control_register_byte(filter_index);
        self.write_register(filter_reg, 0x00)?;

        Ok(())
    }

    /// Reads and returns the operation status
    pub fn read_operation_status(&mut self) -> Result<OperationStatus, CanError<D>> {
        let data = self.read_register(REGISTER_C1CON + 2)?;

        Ok(OperationStatus::from_register(data))
    }

    /// Reads and returns the oscillator status
    pub fn read_oscillator_status(&mut self) -> Result<OscillatorStatus, CanError<D>> {
        let data = self.read_register(REGISTER_OSC + 1)?;

        Ok(OscillatorStatus::from_register(data))
    }

    /// Reads and returns the current clock configuration
    pub fn read_clock_configuration(&mut self) -> Result<ClockConfiguration, CanError<D>> {
        let data = self.read_register(REGISTER_OSC)?;

        Ok(ClockConfiguration::from_register(data))
    }

    /// Enters the given mode, aborts all running transactions
    /// and waits max. 2 ms for the given mode to be reached
    fn enable_mode(&mut self, mode: OperationMode, clock: &CLK, timeout_error: CanError<D>) -> Result<(), CanError<D>> {
        self.write_register(REGISTER_C1CON + 3, mode as u8 | (1 << 3))?;

        let target = clock.try_now()?.checked_add(Milliseconds::new(2)).ok_or(CanError::ClockError)?;

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
    pub fn enable_filter(&mut self, fifo_index: u8, filter_index: u8) -> Result<(), CanError<D>> {
        let filter_control_reg = Self::filter_control_register_byte(filter_index);

        // Filter must be disabled to modify FmBP
        self.disable_filter(filter_index)?;

        // Write index of fifo where the message that matches the filter is stored in
        self.write_register(filter_control_reg, fifo_index)?;

        // Set FLTENm to enable filter
        self.write_register(filter_control_reg, (1 << 7) | fifo_index)?;

        Ok(())
    }

    /// Writes a single register byte
    fn write_register(&mut self, register: u16, value: u8) -> Result<(), SpiError<D>> {
        let mut buffer = self.cmd_buffer(register, Operation::Write);
        buffer[2] = value;

        self.transfer(&mut buffer)?;
        Ok(())
    }

    /// 4-byte SFR write
    fn write32(&mut self, register: u16, value: u32) -> Result<(), SpiError<D>> {
        let mut buffer = [0u8; 6];
        let command = (register & 0x0FFF) | ((Operation::Write as u16) << 12);

        let value_bytes = value.to_le_bytes();

        buffer[0] = (command >> 8) as u8;
        buffer[1] = (command & 0xFF) as u8;
        buffer[2..].copy_from_slice(&value_bytes);

        self.device.write(&buffer).map_err(SpiError::BusError)?;

        Ok(())
    }

    /// Reset internal register to default and switch to Configuration mode
    pub fn reset(&mut self) -> Result<(), CanError<D>> {
        let mut buffer = self.cmd_buffer(0u16, Operation::Reset);
        self.transfer(&mut buffer)?;

        Ok(())
    }

    /// Insert message object in TX FIFO
    fn write_fifo<T, const L: usize>(&mut self, register: u16, message: &TxMessage<T, L>) -> Result<(), CanError<D>>
    where
        T: MessageType<L>,
    {
        self.verify_ram_address(register, message.buff.len())?;

        let mut buffer = [0u8; 10];
        let command = (register & 0x0FFF) | ((Operation::Write as u16) << 12);

        // copy message data into mutable buffer
        let mut data = [0u8; L];
        data[..message.buff.len()].copy_from_slice(&message.buff);

        buffer[0] = (command >> 8) as u8;
        buffer[1] = (command & 0xFF) as u8;
        buffer[2..].copy_from_slice(&message.header.into_bytes());

        for word in buffer[2..].as_chunks_mut::<4>().0 {
            let num = BigEndian::read_u32(word);
            LittleEndian::write_u32(word, num);
        }
        let mut operations = [SpiOperation::Write(&buffer), SpiOperation::Write(&data)];
        self.device.transaction(&mut operations).map_err(SpiError::BusError)?;

        Ok(())
    }

    /// Read message from RX FIFO
    pub(crate) fn read_fifo(&mut self, register: u16, data: &mut [u8]) -> Result<(), CanError<D>> {
        self.verify_ram_address(register, 8 + data.len())?;

        // Skip receive message object header
        let payload_address = register + 8;
        let mut buffer = [0u8; 2];

        let command = (payload_address & 0x0FFF) | ((Operation::Read as u16) << 12);

        buffer[0] = (command >> 8) as u8;
        buffer[1] = (command & 0xFF) as u8;

        let mut operations = [SpiOperation::Write(&buffer), SpiOperation::Read(data)];
        self.device.transaction(&mut operations).map_err(SpiError::BusError)?;

        Ok(())
    }

    /// 4-byte SFR read
    fn read32(&mut self, register: u16) -> Result<u32, CanError<D>> {
        // create cmd buffer (2 bytes cmd+addr)
        let mut buffer = [0u8; 2];
        // payload received buffer
        let mut data = [0u8; 4];
        let command = (register & 0x0FFF) | ((Operation::Read as u16) << 12);

        buffer[0] = (command >> 8) as u8;
        buffer[1] = (command & 0xFF) as u8;

        let mut operations = [SpiOperation::Write(&buffer), SpiOperation::Read(&mut data)];
        self.device.transaction(&mut operations).map_err(SpiError::BusError)?;

        // SFR addresses are at the LSB of the registers
        // so last read byte is the MSB of the register
        // and since bitfield_msb is used, order of bytes is reversed
        let result = u32::from_le_bytes(data);
        Ok(result)
    }

    /// Verify address within RAM bounds
    fn verify_ram_address(&self, addr: u16, data_length: usize) -> Result<(), CanError<D>> {
        if addr < 0x400 || addr as usize + data_length > 0xC00 {
            return Err(CanError::InvalidRamAddress(addr));
        }

        Ok(())
    }

    /// Reads a single register byte
    fn read_register(&mut self, register: u16) -> Result<u8, SpiError<D>> {
        let mut buffer = self.cmd_buffer(register, Operation::Read);

        self.transfer(&mut buffer)
    }

    /// Executes a SPI transfer with three bytes buffer and returns the last byte received
    fn transfer(&mut self, buffer: &mut [u8]) -> Result<u8, SpiError<D>> {
        self.device.transfer_in_place(buffer).map_err(SpiError::BusError)?;

        Ok(buffer[2])
    }

    /// Creates a three byte command buffer for the given register
    fn cmd_buffer(&self, register: u16, operation: Operation) -> [u8; 3] {
        let mut buffer = [0x0u8; 3];
        let command = (register & 0x0FFF) | ((operation as u16) << 12);

        buffer[0] = (command >> 8) as u8;
        buffer[1] = (command & 0xFF) as u8;

        buffer
    }

    /// Returns if the TX/RX fifo not full/empty flag is set
    fn fifo_tfnrfnif(&mut self, fifo_reg_addr: u16) -> Result<bool, CanError<D>> {
        let txfifo_status_byte0 = self.read_register(fifo_reg_addr)?;
        let txfifo_status_reg0 = FifoStatusReg0::from(txfifo_status_byte0);

        Ok(txfifo_status_reg0.tfnrfnif())
    }

    /// Returns true if `TXREQ` bit of TX fifo is cleared i.e. all messages contained are transmitted
    fn txfifo_cleared(&mut self, fifo_ctrl_reg: u16) -> Result<bool, CanError<D>> {
        // read TX FIFO control register byte 1
        let txfifo_control_byte1 = self.read_register(fifo_ctrl_reg)?;
        let txfifo_control_reg = FifoControlReg1::from(txfifo_control_byte1);

        Ok(!txfifo_control_reg.txreq())
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

impl<D: SpiDevice> From<embedded_time::clock::Error> for CanError<D> {
    fn from(_error: embedded_time::clock::Error) -> Self {
        CanError::ClockError
    }
}
