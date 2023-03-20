use crate::config::{
    ClockConfiguration, ClockOutputDivisor, FifoConfiguration, PLLSetting, RetransmissionAttempts, SystemClockDivisor,
};

#[test]
fn test_clock_from_register() {
    assert_eq!(
        ClockOutputDivisor::DivideBy10,
        ClockConfiguration::from_register(0b0110_0100).clock_output
    );
    assert_eq!(
        ClockOutputDivisor::DivideBy4,
        ClockConfiguration::from_register(0b0100_0100).clock_output
    );
    assert_eq!(
        ClockOutputDivisor::DivideBy2,
        ClockConfiguration::from_register(0b0010_0100).clock_output
    );
    assert_eq!(
        ClockOutputDivisor::DivideBy1,
        ClockConfiguration::from_register(0b0000_0100).clock_output
    );

    assert_eq!(
        SystemClockDivisor::DivideBy2,
        ClockConfiguration::from_register(0b0011_0100).system_clock
    );
    assert_eq!(
        SystemClockDivisor::DivideBy1,
        ClockConfiguration::from_register(0b0000_0100).system_clock
    );

    assert!(ClockConfiguration::from_register(0b0011_0100).disable_clock);
    assert!(!ClockConfiguration::from_register(0b0011_0000).disable_clock);

    assert_eq!(
        PLLSetting::TenTimesPLL,
        ClockConfiguration::from_register(0b0011_0101).pll
    );
    assert_eq!(
        PLLSetting::DirectXTALOscillator,
        ClockConfiguration::from_register(0b0000_0100).pll
    );
}

#[test]
fn test_clock_configuration_to_register() {
    assert_eq!(0x0, ClockConfiguration::default().as_register());

    assert_eq!(
        0b0101_0001,
        ClockConfiguration {
            clock_output: ClockOutputDivisor::DivideBy4,
            system_clock: SystemClockDivisor::DivideBy2,
            disable_clock: false,
            pll: PLLSetting::TenTimesPLL
        }
        .as_register()
    );

    assert_eq!(
        0b0110_0100,
        ClockConfiguration {
            clock_output: ClockOutputDivisor::DivideBy10,
            system_clock: SystemClockDivisor::DivideBy1,
            disable_clock: true,
            pll: PLLSetting::DirectXTALOscillator
        }
        .as_register()
    );

    assert_eq!(
        0b0000_0100,
        ClockConfiguration {
            clock_output: ClockOutputDivisor::DivideBy1,
            system_clock: SystemClockDivisor::DivideBy1,
            disable_clock: true,
            pll: PLLSetting::DirectXTALOscillator
        }
        .as_register()
    );

    assert_eq!(
        0b0010_0100,
        ClockConfiguration {
            clock_output: ClockOutputDivisor::DivideBy2,
            system_clock: SystemClockDivisor::DivideBy1,
            disable_clock: true,
            pll: PLLSetting::DirectXTALOscillator
        }
        .as_register()
    );
}

#[test]
fn test_fifo_configuration_as_rx_register() {
    assert_eq!(0b0000_0000, fifo_rx_config(0).as_rx_register());
    assert_eq!(0b0000_0000, fifo_rx_config(1).as_rx_register());

    assert_eq!(0b0000_0001, fifo_rx_config(2).as_rx_register());
    assert_eq!(0b0000_1011, fifo_rx_config(12).as_rx_register());

    assert_eq!(0b0001_1111, fifo_rx_config(32).as_rx_register());
    assert_eq!(0b0001_1111, fifo_rx_config(33).as_rx_register());
}

#[test]
fn test_fifo_configuration_as_tx_register_2() {
    assert_eq!(
        0b0101_0100,
        FifoConfiguration {
            rx_size: 0,
            tx_attempts: RetransmissionAttempts::Unlimited,
            tx_priority: 20,
            ..Default::default()
        }
        .as_tx_register_2()
    );

    assert_eq!(
        0b0000_0000,
        FifoConfiguration {
            rx_size: 0,
            tx_attempts: RetransmissionAttempts::Disabled,
            tx_priority: 0,
            ..Default::default()
        }
        .as_tx_register_2()
    );

    assert_eq!(
        0b0011_1111,
        FifoConfiguration {
            rx_size: 0,
            tx_attempts: RetransmissionAttempts::Three,
            tx_priority: 32,
            ..Default::default()
        }
        .as_tx_register_2()
    );

    assert_eq!(
        0b0001_1111,
        FifoConfiguration {
            rx_size: 0,
            tx_attempts: RetransmissionAttempts::Disabled,
            tx_priority: 33,
            ..Default::default()
        }
        .as_tx_register_2()
    );
}

#[test]
fn test_fifo_configuration_as_tx_register_3() {
    assert_eq!(0b0000_0000, fifo_tx_config(0).as_tx_register_3());
    assert_eq!(0b0000_0000, fifo_tx_config(1).as_tx_register_3());

    assert_eq!(0b0000_0001, fifo_tx_config(2).as_tx_register_3());
    assert_eq!(0b0000_1011, fifo_tx_config(12).as_tx_register_3());

    assert_eq!(0b0001_1111, fifo_tx_config(32).as_tx_register_3());
    assert_eq!(0b0001_1111, fifo_tx_config(33).as_tx_register_3());
}

#[test]
fn test_fifo_configuration_as_tx_register_0() {
    assert_eq!(
        0b1000_0000,
        FifoConfiguration {
            tx_enable: true,
            ..Default::default()
        }
        .as_tx_register_0()
    );

    assert_eq!(
        0b0000_0000,
        FifoConfiguration {
            tx_enable: false,
            ..Default::default()
        }
        .as_tx_register_0()
    );
}

fn fifo_rx_config(rx_size: u8) -> FifoConfiguration {
    FifoConfiguration {
        rx_size,
        ..Default::default()
    }
}

fn fifo_tx_config(tx_size: u8) -> FifoConfiguration {
    FifoConfiguration {
        tx_size,
        ..Default::default()
    }
}
