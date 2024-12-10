#![cfg_attr(not(test), no_std)]
#![cfg_attr(feature = "strict", deny(warnings))]
#![allow(dead_code)]
#![allow(clippy::identity_op)]

//! # Library for MCP2517FD CAN controller
//!
//! Crate currently offers the following features:
//! * CAN2.0 and CAN FD format support
//! * Standard and extended ID formats for CAN frames
//! * `no_std` support
//!
//!## Example
//! For detailed example with rp-pico check [example](https://github.com/atlas-aero/rt-mcp2517/tree/main/example)
//!
//!## CAN TX/RX example
//!
//!```
//!use mcp2517::example::{ExampleClock,ExampleSPIDevice};
//!use mcp2517::can::{MCP2517,CanController};
//!use mcp2517::message::{Can20,TxMessage};
//!use mcp2517::filter::Filter;
//!use mcp2517::config::*;
//!use bytes::Bytes;
//!use embedded_can::{Id,StandardId};
//!
//!let spi_dev = ExampleSPIDevice::default();
//!let clock = ExampleClock::default();
//!
//!let mut controller = MCP2517::new(spi_dev);
//!
//!// configure CAN controller
//!controller
//!    .configure(
//!        &Configuration {
//!            clock: ClockConfiguration {
//!                clock_output: ClockOutputDivisor::DivideBy10,
//!                system_clock: SystemClockDivisor::DivideBy1,
//!                disable_clock: false,
//!                pll: PLLSetting::TenTimesPLL,
//!                 },
//!            fifo: FifoConfiguration {
//!                rx_size: 16,
//!                tx_attempts: RetransmissionAttempts::Three,
//!                tx_priority: 10,
//!                pl_size: PayloadSize::EightBytes,
//!                tx_size: 20,
//!                tx_enable: true,
//!                 },
//!            mode: RequestMode::NormalCANFD,
//!            bit_rate: BitRateConfig{
//!                sys_clk: SysClk::MHz20,
//!                can_speed: CanBaudRate::Kpbs500
//!                },
//!             },
//!        &clock,
//!         ).unwrap();
//!
//!// Create message frame
//!let can_id = Id::Standard(StandardId::new(0x55).unwrap());
//!
//!// Important note: Generic arg for message type for CAN2.0
//!// should be either 4 or 8, the DLC will be based off the
//!// length of the payload buffer. So for a payload of 5 bytes
//!// you can only use Can20::<8> as the message type
//!let message_type = Can20::<8> {};
//!let payload = [0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8];
//!let pl_bytes = Bytes::copy_from_slice(&payload);
//!let can_message = TxMessage::new(message_type, pl_bytes, can_id).unwrap();
//!
//!// Create and set filter object
//!let filter = Filter::new(can_id, 0).unwrap();
//!let _ = controller.set_filter_object(filter);
//!
//!// Transmit CAN message in blocking mode
//!controller.transmit(&can_message,true).unwrap();
//!
//!// Receive CAN message in blocking mode
//!let mut buff = [0u8;8];
//!let result = controller.receive(&mut buff,true);
//!assert!(result.is_ok());
//!assert_eq!(buff,[0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8]);
//!```

extern crate alloc;

pub mod can;
pub mod config;
#[cfg(feature = "example")]
pub mod example;
pub mod filter;
pub mod message;
#[cfg(test)]
pub(crate) mod mocks;
mod registers;
pub mod status;
#[cfg(test)]
mod tests;
