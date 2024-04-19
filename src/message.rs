use bytes::BytesMut;
use embedded_can::Id;
use log::debug;
use modular_bitfield_msb::prelude::*;

pub const STANDARD_IDENTIFIER_MASK: u16 = 0x7FF;
pub const EXTENDED_IDENTIFIER_MASK: u32 = 0x3FFFF;
pub const MAX_PAYLOAD_CAN_2_0: usize = 8;
pub const MAX_PAYLOAD_CAN_FD: usize = 64;

/// Data length code
#[derive(BitfieldSpecifier, Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone)]
#[bits = 4]
pub enum DLC {
    Zero,
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Twelve,
    Sixteen,
    Twenty,
    TwentyFour,
    ThirtyTwo,
    FortyEight,
    SixtyFour,
}
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum DLCError {
    InvalidLength(usize),
}

impl DLC {
    fn from_length(value: usize) -> Result<Self, DLCError> {
        match value {
            0 => Ok(Self::Zero),
            1 => Ok(Self::One),
            2 => Ok(Self::Two),
            3 => Ok(Self::Three),
            4 => Ok(Self::Four),
            5 => Ok(Self::Five),
            6 => Ok(Self::Six),
            7 => Ok(Self::Seven),
            8 => Ok(Self::Eight),
            12 => Ok(Self::Twelve),
            16 => Ok(Self::Sixteen),
            20 => Ok(Self::Twenty),
            24 => Ok(Self::TwentyFour),
            32 => Ok(Self::ThirtyTwo),
            48 => Ok(Self::FortyEight),
            64 => Ok(Self::SixtyFour),
            val => Err(DLCError::InvalidLength(val)),
        }
    }
}

/// Transmit message object header
#[bitfield(bits = 64)]
#[derive(BitfieldSpecifier, Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Default)]
pub struct TxHeader {
    // T0
    #[skip]
    __: B2,
    pub sid11: bool,
    pub extended_identifier: B18,
    pub standard_identifier: B11,
    // T1
    #[skip]
    __: B16,
    pub sequence: B7,
    pub error_status_indicator: bool,
    pub fd_frame: bool,
    pub bit_rate_switch: bool,
    pub remote_transmission_request: bool,
    pub identifier_extension_flag: bool,
    pub data_length_code: DLC,
}

impl TxHeader {}

/// Transmit Message Object
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct TxMessage {
    pub(crate) header: TxHeader,
    pub(crate) buff: BytesMut,
    pub(crate) length: usize,
}

impl TxMessage {
    pub fn new(identifier: Id, data: &[u8], can_fd: bool, bitrate_switch: bool) -> Result<Self, DLCError> {
        let mut header = TxHeader::new();
        let mut payload_length = data.len();

        if can_fd {
            header.set_fd_frame(true);
            if data.len() > MAX_PAYLOAD_CAN_FD {
                debug!("Maximum of 64 data bytes allowed for CANFD message. Current size: {payload_length}");
                return Err(DLCError::InvalidLength(data.len()));
            }
            if bitrate_switch {
                header.set_bit_rate_switch(true);
            }
        } else if data.len() > MAX_PAYLOAD_CAN_2_0 {
            debug!("Maximum of 8 data bytes allowed for CAN2.0 message. Current size: {payload_length}");
            return Err(DLCError::InvalidLength(data.len()));
        }
        // make sure length divisible by four (word size)
        let length = (payload_length + 3) & !3;

        let mut bytes = BytesMut::with_capacity(payload_length);
        bytes.extend_from_slice(data);

        while let Err(DLCError::InvalidLength(_)) = DLC::from_length(payload_length) {
            payload_length += 1;
        }

        header.set_data_length_code(DLC::from_length(payload_length)?);

        match identifier {
            Id::Standard(sid) => header.set_standard_identifier(sid.as_raw()),
            Id::Extended(eid) => {
                header.set_extended_identifier(eid.as_raw() & EXTENDED_IDENTIFIER_MASK);
                header.set_standard_identifier((eid.as_raw() >> 18) as u16 & STANDARD_IDENTIFIER_MASK);
                header.set_identifier_extension_flag(true);
            }
        }
        Ok(TxMessage {
            header,
            buff: bytes,
            length,
        })
    }
}
