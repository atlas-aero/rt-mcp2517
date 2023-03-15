use embedded_hal::blocking::spi::Transfer;
use embedded_hal::digital::v2::OutputPin;

/// Main MCP2517 CAN controller device
pub struct Controller<B: Transfer<u8>, CS: OutputPin> {
    /// SPI bus
    bus: B,

    /// CS pin
    pin_cs: CS,
}

impl<B: Transfer<u8>, CS: OutputPin> Controller<B, CS> {
    pub fn new(bus: B, pin_cs: CS) -> Self {
        Self { bus, pin_cs }
    }
}
