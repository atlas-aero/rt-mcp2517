use crate::config::{ClockConfiguration, ClockOutputDivisor, PLLSetting, SystemClockDivisor};

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
