#![no_std]
#![no_main]

pub mod clock;
pub mod heap;
pub mod mutex;

use crate::clock::SystemClock;
use crate::heap::Heap;
use bytes::Bytes;
use cortex_m::asm::delay;
use embedded_can::{Id, StandardId};
use embedded_hal::delay::DelayNs;
use hal::clocks::Clock;
use hal::fugit::RateExtU32;
use hal::pac;
use log::info;
use mcp2517::can::Controller;
use mcp2517::config::{
    ClockConfiguration, ClockOutputDivisor, Configuration, FifoConfiguration, PLLSetting, PayloadSize, RequestMode,
    RetransmissionAttempts, SystemClockDivisor,
};
use mcp2517::filter::Filter;
use mcp2517::message::{Can20, TxMessage};
use panic_halt as _;
use rp2040_hal as hal;

#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_GENERIC_03H;

const XTAL_FREQ_HZ: u32 = 12_000_000u32;

#[rp2040_hal::entry]
fn main() -> ! {
    Heap::init();

    let mut pac = pac::Peripherals::take().unwrap();
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    // Configure the clocks
    let clocks = hal::clocks::init_clocks_and_plls(
        XTAL_FREQ_HZ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .unwrap();

    let sio = hal::Sio::new(pac.SIO);

    let pins = hal::gpio::Pins::new(pac.IO_BANK0, pac.PADS_BANK0, sio.gpio_bank0, &mut pac.RESETS);

    let spi_mosi = pins.gpio7.into_function::<hal::gpio::FunctionSpi>();
    let spi_miso = pins.gpio4.into_function::<hal::gpio::FunctionSpi>();
    let spi_sclk = pins.gpio6.into_function::<hal::gpio::FunctionSpi>();
    let spi = hal::spi::Spi::<_, _, _, 8>::new(pac.SPI0, (spi_mosi, spi_miso, spi_sclk));

    // Exchange the uninitialised SPI driver for an initialised one
    let spi = spi.init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        16.MHz(),
        embedded_hal::spi::MODE_0,
    );

    let mut timer = hal::Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);
    let sys_clk = SystemClock::default();
    sys_clk.initialize(timer);

    // Configure GPIO5 as an CS pin
    let pin_cs = pins.gpio5.into_push_pull_output();

    let mut can_controller: Controller<_, _, SystemClock> = Controller::new(spi, pin_cs);

    // Setup clk config
    let clk_config = ClockConfiguration {
        clock_output: ClockOutputDivisor::DivideBy1,
        system_clock: SystemClockDivisor::DivideBy1,
        pll: PLLSetting::DirectXTALOscillator,
        disable_clock: false,
    };

    // Setup fifo config
    let fifo_config = FifoConfiguration {
        rx_size: 1,
        tx_size: 1,
        pl_size: PayloadSize::EightBytes,
        tx_priority: 32,
        tx_enable: true,
        tx_attempts: RetransmissionAttempts::Unlimited,
    };

    // Setup CAN Controller config
    let config = Configuration {
        clock: clk_config,
        fifo: fifo_config,
        mode: RequestMode::InternalLoopback,
    };

    let _ = can_controller.configure(&config, &sys_clk);

    let can_id = Id::Standard(StandardId::new(0x55).unwrap());

    // Create filter object for RX
    let filter = Filter::new(can_id, 0).unwrap();
    let _ = can_controller.set_filter_object(filter);

    // Create message frame
    let message_type = Can20 {};
    let payload = [1, 2, 3, 4, 5, 6, 7, 8];
    let pl_bytes = Bytes::copy_from_slice(&payload);
    let can_message = TxMessage::new(message_type, pl_bytes, can_id).unwrap();

    let mut receive_buffer = [0u8; 8];

    loop {
        can_controller.transmit(&can_message).unwrap();
        info!("Message sent");

        match can_controller.receive(&mut receive_buffer).unwrap() {
            Ok(_) => info!("message received {receive_buffer:?}"),
            Err(e) => info!("Error while attempting to read message: {e:?}"),
        }

        timer.delay_ms(500);
    }
}
