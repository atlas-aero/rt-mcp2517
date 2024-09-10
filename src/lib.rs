#![cfg_attr(not(test), no_std)]
#![cfg_attr(feature = "strict", deny(warnings))]
#![allow(dead_code)]
#![allow(clippy::identity_op)]

//! # Library for MCP2517FD CAN controller
//!
//! Crate currently offer the following features:
//! * CAN2.0 and CAN FD format support
//! * Standard and extended ID formats for CAN frames
//! * no_std support
//!
//!## Example
//! For detailed example with rp-pico check [example](https://github.com/atlas-aero/rt-mcp2517/tree/main/example)
//!
//!## CAN Tx/Rx example
//!
//!```
//!use mcp2517::example::{ExampleClock,ExampleCSPin,ExampleSPIBus};
//!use mcp2517::can::Controller;
//!use mcp2517::message::{Can20,TxMessage};
//!use mcp2517::filter::Filter;
//!use mcp2517::config::*;
//!use bytes::Bytes;
//!use embedded_can::{Id,StandardId};
//!
//!let cs_pin = ExampleCSPin{};
//!let spi_bus = ExampleSPIBus::default();
//!let clock = ExampleClock::default();
//!
//!let mut controller = Controller::new(spi_bus, cs_pin);
//! // configure CAN controller
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
//! // Create message frame
//!let can_id = Id::Standard(StandardId::new(0x55).unwrap());
//!let message_type = Can20::<8> {};
//!let payload = [1, 2, 3, 4, 5, 6, 7, 8];
//!let pl_bytes = Bytes::copy_from_slice(&payload);
//!let can_message = TxMessage::new(message_type, pl_bytes, can_id).unwrap();
//!// Create and set filter object
//!let filter = Filter::new(can_id, 0).unwrap();
//!let _ = controller.set_filter_object(filter);
//!// Transmit CAN message
//!controller.transmit(&can_message).unwrap();
//!
//!let mut buff = [0u8;8];
//!// Receive CAN message
//!let result = controller.receive(&mut buff);
//!assert!(result.is_ok());
//!assert_eq!(buff,[1,2,3,4,5,6,7,8]);
//!```

extern crate alloc;

pub mod can;
pub mod config;
pub mod status;

pub mod filter;
pub mod message;

pub mod example;
#[cfg(test)]
pub(crate) mod mocks;
mod registers;
#[cfg(test)]
mod tests;
