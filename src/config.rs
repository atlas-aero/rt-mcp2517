/// Entire configuration currently supported
#[derive(Default, Clone, Debug)]
pub struct Configuration {
    pub clock: ClockConfiguration,
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
