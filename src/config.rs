//!
//!
//! # CAN Module configuration
//! The [Configuration] struct provides an abstraction for configuring the CAN module registers.
//! ## Fifo configuration
//! The following example shows a FIFO buffer configuration. At the moment, there is one TX Fifo and one
//! Rx Fifo. The configuration sets the max payload size of messages transmitted/received in both Fifo buffers
//! to 8 bytes. The number of message the Rx Fifo buffer can hold is 10 while it is 32 for the Tx Fifo.
//!
//! The priority for the messages in the Tx Fifo are given the highest possible priority (32) and the retransmission
//! attemps are set to be unlimited.
//!```
//! use mcp2517::config::{FifoConfiguration,PayloadSize,RetransmissionAttempts};
//!
//! let fifo_config = FifoConfiguration{
//! pl_size:PayloadSize::EightBytes,
//! rx_size:10,
//! tx_attempts:RetransmissionAttempts::Unlimited,
//! tx_enable:true,
//! tx_priority:32,
//! tx_size:32,
//! };
//!```
//! ## Clock configuration
//! The CAN system clock is determined through setting the `system_clock` and `pll`. In this example,
//! the pll setting used is to directly use the crystal oscillator without any multiplication, and the
//! `system_clock` divisor is set to 1. Meaning the frequency of the SYSCLK matches the crystal used.
//! The `clock_output` divisior is set to 2. So that the CLKO outputs half the freq of the crystal.
//! Refer to the [datasheet](https://ww1.microchip.com/downloads/en/DeviceDoc/MCP2517FD-External-CAN-FD-Controller-with-SPI-Interface-20005688B.pdf) section 5.0
//!
//!```
//! use mcp2517::config::{ClockConfiguration, ClockOutputDivisor, PLLSetting, SystemClockDivisor};
//!
//! let clock_config = ClockConfiguration{
//! clock_output:ClockOutputDivisor::DivideBy2,
//! system_clock:SystemClockDivisor::DivideBy1,
//! pll:PLLSetting::DirectXTALOscillator,
//! disable_clock:false,
//! };
//!```
//!
//!
use crate::status::OperationMode;

/// Entire configuration currently supported
#[derive(Default, Clone, Debug)]
pub struct Configuration {
    /// Oscillator/Clock configuration
    pub clock: ClockConfiguration,

    /// TX/RX FIFO configuration
    pub fifo: FifoConfiguration,

    /// Target request/operation mode
    pub mode: RequestMode,
}

/// Oscillator/Clock configuration
#[derive(Copy, Clone, Debug, Default)]
pub struct ClockConfiguration {
    /// Divisor for clock output
    pub clock_output: ClockOutputDivisor,

    /// Divisor for system clock
    pub system_clock: SystemClockDivisor,

    /// Disable clock/oscillator?
    pub disable_clock: bool,

    /// PLL configuration
    pub pll: PLLSetting,
}

impl ClockConfiguration {
    /// Maps register values to configuration
    pub(crate) fn from_register(register: u8) -> Self {
        Self {
            clock_output: ClockOutputDivisor::from_register(register),
            system_clock: SystemClockDivisor::from_register(register),
            disable_clock: register & (1 << 2) != 0,
            pll: PLLSetting::from_register(register),
        }
    }

    /// Encodes the configuration to register byte
    pub(crate) fn as_register(&self) -> u8 {
        let mut register = 0x0;

        register |= (self.clock_output as u8) << 5;
        register |= (self.system_clock as u8) << 4;
        register |= (self.disable_clock as u8) << 2;
        register |= self.pll as u8;

        register
    }
}

/// Divisor for clock output
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ClockOutputDivisor {
    DivideBy10 = 0b11,
    DivideBy4 = 0b10,
    DivideBy2 = 0b01,
    DivideBy1 = 0b00,
}

impl Default for ClockOutputDivisor {
    fn default() -> Self {
        Self::DivideBy1
    }
}

impl ClockOutputDivisor {
    /// Maps register values to configuration
    pub(crate) fn from_register(register: u8) -> Self {
        match register >> 5 {
            0b11 => Self::DivideBy10,
            0b10 => Self::DivideBy4,
            0b01 => Self::DivideBy2,
            _ => Self::DivideBy1,
        }
    }
}

/// Divisor for system clock
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SystemClockDivisor {
    DivideBy2 = 0b1,
    DivideBy1 = 0b0,
}

impl Default for SystemClockDivisor {
    fn default() -> Self {
        Self::DivideBy1
    }
}

impl SystemClockDivisor {
    /// Maps register values to configuration
    pub(crate) fn from_register(register: u8) -> Self {
        if register & (1 << 4) != 0 {
            Self::DivideBy2
        } else {
            Self::DivideBy1
        }
    }
}

/// PLL configuration
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PLLSetting {
    /// System clock from 10x PLL
    TenTimesPLL = 0b1,
    /// System clock comes directly from XTAL oscillator
    DirectXTALOscillator = 0b0,
}

impl Default for PLLSetting {
    fn default() -> Self {
        Self::DirectXTALOscillator
    }
}

impl PLLSetting {
    /// Maps register values to configuration
    pub(crate) fn from_register(register: u8) -> Self {
        if register & 1 != 0 {
            Self::TenTimesPLL
        } else {
            Self::DirectXTALOscillator
        }
    }
}

/// Transmit and receive FIFO configuration
#[derive(Copy, Clone, Debug)]
pub struct FifoConfiguration {
    /// Receive FIFO size in message: 0 - 32.
    /// Value is limited to 32 messages if a higher value is given.
    pub rx_size: u8,

    /// Number of retransmission attempts
    pub tx_attempts: RetransmissionAttempts,

    /// Transmission priority of FIFO queue (0 = Lowest, 32 = Highest)
    /// Value is limited to 32 if a higher value is given
    pub tx_priority: u8,

    /// Transmission FIFO size in message: 0 - 32.
    /// Value is limited to 32 messages if a higher value is given.
    pub tx_size: u8,

    /// Number of payload bytes in message
    pub pl_size: PayloadSize,

    /// Enables/Disables TX FIFO
    pub tx_enable: bool,
}

/// Permitted sizes of the message payload for a FIFO
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PayloadSize {
    EightBytes = 0b000,
    TwelveBytes = 0b001,
    SixteenBytes = 0b010,
    TwentyBytes = 0b011,
    TwentyFourBytes = 0b100,
    ThirtyTwoBytes = 0b101,
    FortyEightBytes = 0b110,
    SixtyFourBytes = 0b111,
}

impl Default for FifoConfiguration {
    fn default() -> Self {
        Self {
            rx_size: 32,
            tx_attempts: RetransmissionAttempts::default(),
            tx_priority: 0,
            tx_size: 32,
            pl_size: PayloadSize::EightBytes,
            tx_enable: true,
        }
    }
}

impl FifoConfiguration {
    /// Encodes the configuration for the third RX fifo control register byte
    pub(crate) fn as_rx_register_3(&self) -> u8 {
        (Self::limit_size(self.rx_size) - 1) | ((self.pl_size as u8) << 5)
    }

    /// Encodes the configuration for the first TX configuration register byte
    pub(crate) fn as_tx_register_0(&self) -> u8 {
        // bit 7 -> tx enable
        // bit 0 -> tx fifo not full interrupt flag enable
        match self.tx_enable {
            true => 0b1000_0000,
            false => 0b0000_0000,
        }
    }

    /// Encodes the configuration for the third TX configuration register byte
    pub(crate) fn as_tx_register_2(&self) -> u8 {
        ((self.tx_attempts as u8) << 5) | self.tx_priority.min(31)
    }

    /// Encodes the configuration for the fourth TX configuration register byte
    pub(crate) fn as_tx_register_3(&self) -> u8 {
        (Self::limit_size(self.tx_size) - 1) | ((self.pl_size as u8) << 5)
    }

    /// Limits the size to valid values
    fn limit_size(size: u8) -> u8 {
        size.clamp(1, 32)
    }
}

/// Number of retransmission attempts
#[derive(Copy, Clone, Debug)]
pub enum RetransmissionAttempts {
    Disabled = 0b00,
    Three = 0b01,
    Unlimited = 0b10,
}

impl Default for RetransmissionAttempts {
    fn default() -> Self {
        Self::Unlimited
    }
}

/// Request mode. This is basically a subset of operation mode, filtered to request modes
#[derive(Copy, Clone, Debug)]
pub enum RequestMode {
    /// Normal CAN FD mode, supports mixing of CAN FDC can classic CAN 2.0 frames
    NormalCANFD,
    /// Internal loop back mode
    InternalLoopback,
    /// External loop back mode
    ExternalLoopback,
    /// Listen only mode
    ListenOnly,
    /// CAN 2.0 mode, possible error frames on CAN FD frames
    NormalCAN2_0,
}

impl Default for RequestMode {
    fn default() -> Self {
        Self::NormalCANFD
    }
}

impl RequestMode {
    pub(crate) fn to_operation_mode(self) -> OperationMode {
        match self {
            RequestMode::NormalCANFD => OperationMode::NormalCANFD,
            RequestMode::InternalLoopback => OperationMode::InternalLoopback,
            RequestMode::ExternalLoopback => OperationMode::ExternalLoopback,
            RequestMode::ListenOnly => OperationMode::ListenOnly,
            RequestMode::NormalCAN2_0 => OperationMode::NormalCAN2_0,
        }
    }
}
