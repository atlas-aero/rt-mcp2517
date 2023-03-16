///  Operation status read from C1CON register
#[derive(Copy, Clone, Debug)]
pub struct OperationStatus {
    /// Current operation mode
    pub mode: OperationMode,

    /// True if TXQ is enabled an reserves space in RAM
    pub txq_reserved: bool,

    /// True if transmitted messages are stored in TEF and RAM space is reserved
    pub store_transmit_event: bool,

    /// True => Transition to listen-only mode on system error bit
    /// False => Transition to restricted-operation mode on system error bit
    pub error_trans_listen_only_mode: bool,

    /// True => ESI is transmitted recessive when ESI of message is high or CAN controller error passive
    /// False => ESI reflects error stratus of CAN controller
    pub transmit_esi_gateway: bool,

    /// True => Restricted retransmission attempts. TXAT is used
    /// False => Unlimited number of retransmission attempts, TXAT will be ignored
    pub restrict_retransmission: bool,
}

impl OperationStatus {
    pub(crate) fn from_register(register: u8) -> Self {
        Self {
            mode: OperationMode::from_register(register),
            txq_reserved: register & (1 << 4) != 0,
            store_transmit_event: register & (1 << 3) != 0,
            error_trans_listen_only_mode: register & (1 << 2) != 0,
            transmit_esi_gateway: register & (1 << 1) != 0,
            restrict_retransmission: register & 1 != 0,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum OperationMode {
    /// Module is in normal CAN FD mode, supports mixing of CAN FDC can classic CAN 2.0 frames
    NormalCANFD = 0b000,
    /// Module is in sleep mode
    Sleep = 0b001,
    /// Module is in internal loopback mode
    InternalLoopback = 0b010,
    /// Module is in listen-only mode
    ListenOnly = 0b011,
    /// Module is in configuration mode
    Configuration = 0b100,
    /// Module is in external loopback mode
    ExternalLoopback = 0b101,
    /// Module is in normal CAN 2.0 mode, possible error frames on CAN FD frames
    NormalCAN2_0 = 0b110,
    /// Module is in restricted operation mode
    RestrictedOperation = 0b111,
}

impl OperationMode {
    pub(crate) fn from_register(register: u8) -> Self {
        match register >> 5 {
            0b000 => Self::NormalCANFD,
            0b001 => Self::Sleep,
            0b010 => Self::InternalLoopback,
            0b011 => Self::ListenOnly,
            0b100 => Self::Configuration,
            0b101 => Self::ExternalLoopback,
            0b110 => Self::NormalCAN2_0,
            _ => Self::RestrictedOperation,
        }
    }
}
