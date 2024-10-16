# rt-mcp2517

Rust no_std library
for [MCP2517FD](https://ww1.microchip.com/downloads/en/DeviceDoc/MCP2517FD-External-CAN-FD-Controller-with-SPI-Interface-20005688B.pdf)
CAN controller

Crate currently offers the following features:

* CAN2.0 and CAN FD format support
* Standard and extended ID formats for CAN frames
* `no_std` support

## Example

````Rust
use mcp2517::example::{ExampleClock, ExampleCSPin, ExampleSPIBus};
use mcp2517::can::{MCP2517, CanController};
use mcp2517::message::{Can20, TxMessage};
use mcp2517::filter::Filter;
use mcp2517::config::*;
use bytes::Bytes;
use embedded_can::{Id, StandardId};

let cs_pin = ExampleCSPin{};
let spi_bus = ExampleSPIBus::default ();
let clock = ExampleClock::default ();

let mut controller = MCP2517::new(spi_bus, cs_pin);

// configure CAN controller
controller
    .configure(
        &Configuration {
            clock: ClockConfiguration {
                clock_output: ClockOutputDivisor::DivideBy10,
                system_clock: SystemClockDivisor::DivideBy1,
                disable_clock: false,
                pll: PLLSetting::TenTimesPLL,
                 },
            fifo: FifoConfiguration {
                rx_size: 16,
                tx_attempts: RetransmissionAttempts::Three,
                tx_priority: 10,
                pl_size: PayloadSize::EightBytes,
                tx_size: 20,
                tx_enable: true,
                 },
            mode: RequestMode::NormalCANFD,
            bit_rate: BitRateConfig{
                sys_clk: SysClk::MHz20,
                can_speed: CanBaudRate::Kpbs500
                },
             },
        &clock,
         ).unwrap();

// Create message frame
let can_id = Id::Standard(StandardId::new(0x55).unwrap());

// Important note: Generic arg for message type for CAN2.0
// should be either 4 or 8, the DLC will be based off the
// length of the payload buffer. So for a payload of 5 bytes
// you can only use Can20::<8> as the message type
let message_type = Can20::<8 > {};
let payload = [0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8];
let pl_bytes = Bytes::copy_from_slice( & payload);
let can_message = TxMessage::new(message_type, pl_bytes, can_id).unwrap();

// Create and set filter object
let filter = Filter::new(can_id, 0).unwrap();
let _ = controller.set_filter_object(filter);

// Transmit CAN message
controller.transmit( & can_message).unwrap();

// Receive CAN message
let mut buff = [0u8; 8];
let result = controller.receive( & mut buff);
assert!(result.is_ok());
assert_eq!(buff, [0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8]);
````

## Development

Any form of support is greatly appreciated. Feel free to create issues and PRs.
See [DEVELOPMENT](DEVELOPMENT.md) for more details.  

## License
Licensed under either of

* Apache License, Version 2.0, (LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license (LICENSE-MIT or http://opensource.org/licenses/MIT)
at your option.

Each contributor agrees that his/her contribution covers both licenses.
