use crate::config::{ClockConfiguration, ClockOutputDivisor, FifoConfiguration, PLLSetting, SystemClockDivisor};

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
fn test_fife_configuration_as_rx_register() {
    assert_eq!(0b0000_0000, FifoConfiguration { rx_size: 0 }.as_rx_register());
    assert_eq!(0b0000_0000, FifoConfiguration { rx_size: 1 }.as_rx_register());

    assert_eq!(0b0000_0001, FifoConfiguration { rx_size: 2 }.as_rx_register());
    assert_eq!(0b0000_1011, FifoConfiguration { rx_size: 12 }.as_rx_register());

    assert_eq!(0b0001_1111, FifoConfiguration { rx_size: 32 }.as_rx_register());
    assert_eq!(0b0001_1111, FifoConfiguration { rx_size: 33 }.as_rx_register());
}
