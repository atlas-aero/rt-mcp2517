#![no_std]
#![no_main]
#![allow(static_mut_refs)]
extern crate alloc;

pub mod clock;
pub mod heap;
pub mod mutex;

use crate::clock::SystemClock;
use crate::heap::Heap;
use bytes::Bytes;
use core::cell::RefCell;
use core::fmt::Write;
use embedded_can::{Id, StandardId};
use embedded_hal::delay::DelayNs;
use fugit::RateExtU32;
use mcp2517::can::{CanController, MCP2517};
use mcp2517::config::{
    BitRateConfig, ClockConfiguration, ClockOutputDivisor, Configuration, FifoConfiguration, PLLSetting, RequestMode,
    SystemClockDivisor,
};
use mcp2517::filter::Filter;
use mcp2517::message::{Can20, TxMessage};
use panic_halt as panic;
use rp_pico as bsp;

use bsp::{
    entry,
    hal::{
        clocks::{init_clocks_and_plls, Clock},
        gpio::FunctionSpi,
        pac,
        sio::Sio,
        uart::*,
        watchdog::Watchdog,
        Spi, Timer,
    },
};
use embedded_hal_bus::spi::{NoDelay, RefCellDevice};

const XTAL_FREQ_HZ: u32 = 12_000_000u32;

#[entry]
fn main() -> ! {
    Heap::init();

    let mut pac = pac::Peripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);

    // Configure the clocks
    let clocks = init_clocks_and_plls(
        XTAL_FREQ_HZ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .unwrap();

    let sio = Sio::new(pac.SIO);

    let pins = bsp::Pins::new(pac.IO_BANK0, pac.PADS_BANK0, sio.gpio_bank0, &mut pac.RESETS);

    let spi_mosi = pins.gpio11.into_function::<FunctionSpi>();
    let spi_miso = pins.gpio12.into_function::<FunctionSpi>();
    let spi_sclk = pins.gpio10.into_function::<FunctionSpi>();
    let spi = Spi::<_, _, _, 8>::new(pac.SPI1, (spi_mosi, spi_miso, spi_sclk));

    // Exchange the uninitialised SPI driver for an initialised one
    let spi = spi.init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        1.MHz(),
        embedded_hal::spi::MODE_0,
    );

    let mut timer = Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);
    let sys_clk = SystemClock::default();
    sys_clk.initialize(timer);

    // Configure GPIO13 as an CS pin
    let pin_cs = pins.gpio13.into_push_pull_output();

    // Enable uart to print to terminal
    let mut uart = bsp::hal::uart::UartPeripheral::new(
        pac.UART0,
        (pins.gpio0.into_function(), pins.gpio1.into_function()),
        &mut pac.RESETS,
    )
    .enable(
        UartConfig::new(9600.Hz(), DataBits::Eight, None, StopBits::One),
        clocks.peripheral_clock.freq(),
    )
    .unwrap();

    let spi_bus = RefCell::new(spi);

    let device = RefCellDevice::new(&spi_bus, pin_cs, NoDelay).unwrap();

    let mut can_controller: MCP2517<_, _> = MCP2517::new(device);

    // Setup clk config
    let clk_config = ClockConfiguration {
        clock_output: ClockOutputDivisor::DivideBy1,
        system_clock: SystemClockDivisor::DivideBy2,
        pll: PLLSetting::DirectXTALOscillator,
        disable_clock: false,
    };

    // Setup fifo config
    let fifo_config = FifoConfiguration::default();

    // Setup CAN Controller config
    let config = Configuration {
        clock: clk_config,
        fifo: fifo_config,
        mode: RequestMode::InternalLoopback,
        bit_rate: BitRateConfig::default(),
    };

    let _ = can_controller.reset();
    if let Err(_) = can_controller.configure(&config, &sys_clk) {
        panic!()
    }

    let can_id = Id::Standard(StandardId::new(0x55).unwrap());

    // Create filter object for RX
    let mut filter = Filter::new(can_id, 0).unwrap();
    // Set mask to match if only 2 LSB of ID match with filter
    filter.set_mask_standard_id(0xFF);
    let _ = can_controller.set_filter_object(filter);
    // Create message frame
    let payload_8 = [0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8];
    let message_type = Can20::<8> {};
    let pl_8bytes = Bytes::copy_from_slice(&payload_8);
    let can_message = TxMessage::new(message_type, pl_8bytes, can_id).unwrap();

    let mut receive_buffer = [0u8; 8];
    loop {
        let _ = can_controller.transmit(&can_message, true);
        uart.write_raw(b"can message sent\n\r").unwrap();

        timer.delay_ms(500);

        match can_controller.receive(&mut receive_buffer, true) {
            Ok(_) => {
                uart.write_fmt(format_args!("can message received\n\r")).unwrap();

                for val in receive_buffer {
                    uart.write_fmt(format_args!("{val}\n\r")).unwrap();
                }
            }
            Err(_) => uart.write_fmt(format_args!("error reading message")).unwrap(),
        }

        timer.delay_ms(500);
    }
}
