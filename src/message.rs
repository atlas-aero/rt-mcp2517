//!# CAN Message
//! This library supports both CAN2.0 (up two 8 data bytes per CAN Frame)
//! and CAN FD (up to 64 data bytes per CAN frame)
//! formats with both standard and extended frame ID formats
//!
//! ## Message construction example
//! ```
//!use bytes::Bytes;
//!use mcp2517::message::{Can20,TxMessage};
//!use embedded_can::{Id,StandardId};
//!
//! // Frame ID
//! let message_id = Id::Standard(StandardId::new(0x123).unwrap());
//! // Set message type to CAN2.0 with 8 data bytes
//! let message_type = Can20::<8>{};
//! // Create payload buffer
//! let payload = [1,2,3,4,5,6,7,8];
//! // Create Bytes object
//! let bytes = Bytes::copy_from_slice(&payload);
//! // Create message object
//! let tx_message = TxMessage::new(message_type,bytes,message_id).unwrap();
//!```
//!

use bytes::Bytes;
use embedded_can::{ExtendedId, Id, StandardId};
use log::debug;
use modular_bitfield_msb::prelude::*;

pub const STANDARD_IDENTIFIER_MASK: u16 = 0x7FF;

pub const EXTENDED_IDENTIFIER_MASK: u32 = 0x3FFFF;

pub const MAX_PAYLOAD_CAN_2_0: usize = 8;

pub const MAX_PAYLOAD_CAN_FD: usize = 64;

/// Data length code
#[derive(BitfieldSpecifier, Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone)]
#[allow(clippy::upper_case_acronyms)]
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

/// Invalid data length code error
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
    /// standard ID in FD mode can be extended to 12 bits if sid11 is set
    pub sid11: bool,
    /// 18 lsb of extended ID
    pub extended_identifier: B18,
    /// standard ID bits or msb 11 bits of extended ID
    pub standard_identifier: B11,
    // T1
    #[skip]
    __: B16,
    /// Sequence keeping track of transmitted messages in Transmit Event FIFO
    pub sequence: B7,
    /// In normal ESI mode, set if node is error passive, cleared if node is error active
    pub error_status_indicator: bool,
    /// Bit distinguishing between CAN and CAN FD formats
    pub fd_frame: bool,
    /// Enables bit rate switching in CAN FD frames
    pub bit_rate_switch: bool,
    /// Set if the frame is a RTR frame
    pub remote_transmission_request: bool,
    /// Set if extended ID is used
    pub identifier_extension_flag: bool,
    /// 4 bits identifying the payload length
    pub data_length_code: DLC,
}

pub trait MessageType<const L: usize> {
    /// Setup CAN message header depending on message type
    fn setup_header(&self, header: &mut TxHeader, payload_length: usize) -> Result<(), DLCError>;
}

/// CAN 2.0 message type where L is the number  //  if payload_length > MAX_LENGTH {
#[derive(Debug, Copy, Clone)]
pub struct Can20<const L: usize> {}

impl<const L: usize> MessageType<L> for Can20<L> {
    fn setup_header(&self, _header: &mut TxHeader, payload_length: usize) -> Result<(), DLCError> {
        if L > 8 || payload_length > 8 {
            let max = payload_length.max(L);
            debug!("Maximum of 8 bytes allowed. Current size: {max} bytes");
            return Err(DLCError::InvalidLength(max));
        }
        Ok(())
    }
}

/// CAN FD message type where L is number data bytes
#[derive(Debug, Copy, Clone)]
pub struct CanFd<const L: usize> {
    pub bitrate_switch: bool,
}

impl<const L: usize> MessageType<L> for CanFd<L> {
    fn setup_header(&self, header: &mut TxHeader, payload_length: usize) -> Result<(), DLCError> {
        if L > 64 || payload_length > 64 {
            let max = payload_length.max(L);
            debug!("Maximum of 64 bytes allowed. Current size: {max} bytes");
            return Err(DLCError::InvalidLength(max));
        }

        header.set_bit_rate_switch(self.bitrate_switch);
        header.set_fd_frame(true);
        Ok(())
    }
}

/// Transmit Message Object
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct TxMessage<T: MessageType<L>, const L: usize> {
    /// first 2 bytes of Transmit Message Object
    pub(crate) header: TxHeader,
    /// Payload bytes of Message Object
    pub(crate) buff: Bytes,
    /// CAN message type (CAN 2.0 or CAN FD)
    pub(crate) message_type: T,
}

impl<T: MessageType<L>, const L: usize> TxMessage<T, L> {
    pub fn new(message_type: T, data: Bytes, identifier: Id) -> Result<Self, DLCError> {
        let mut header = TxHeader::new();

        let mut payload_length = data.len();

        message_type.setup_header(&mut header, payload_length)?;

        // length used to choose the next supported DLC
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
            buff: data,
            message_type,
        })
    }
}

/// Receive message object header
#[bitfield(bits = 64)]
#[derive(Default, PartialEq, Eq, Debug)]
#[repr(u64)]
pub struct RxHeader {
    // R0
    #[skip]
    __: B2,
    /// In FD mode the standard ID can be extended to 12 bit using r1
    sid11: bool,
    /// Extended Identifier
    extended_identifier: B18,
    /// Standard Identifier
    standard_identifier: B11,
    #[skip]
    __: B16,
    /// Filter Hit, number of filter that matched
    filter_hit: B5,
    #[skip]
    __: B2,
    /// Error Status Indicator
    error_status_indicator: bool,
    /// FD Frame; distinguishes between CAN and CAN FD formats
    fd_frame: bool,
    /// Bit Rate Switch; indicates if data bit rate was switched
    bit_rate_switch: bool,
    /// Remote Transmission Request; not used in CAN FD
    remote_transmission_request: bool,
    /// Identifier Extension Flag; distinguishes between base and extended format
    identifier_extension_flag: bool,
    /// Data Length Code
    data_length_code: DLC,
}

impl RxHeader {
    fn get_id(&self) -> Id {
        if self.identifier_extension_flag() {
            let id = ((self.standard_identifier() as u32) << 18) | (self.extended_identifier());
            let extended_id = ExtendedId::new(id);
            Id::Extended(extended_id.unwrap())
        } else {
            let id = StandardId::new(self.standard_identifier());
            Id::Standard(id.unwrap())
        }
    }

    #[cfg(test)]
    pub fn new_test_cfg(identifier: Id) -> Self {
        match identifier {
            Id::Extended(eid) => Self::new()
                .with_data_length_code(DLC::Eight)
                .with_standard_identifier((eid.as_raw() >> 18) as u16 & STANDARD_IDENTIFIER_MASK)
                .with_extended_identifier(eid.as_raw() & EXTENDED_IDENTIFIER_MASK)
                .with_identifier_extension_flag(true),
            Id::Standard(sid) => Self::new()
                .with_data_length_code(DLC::Eight)
                .with_standard_identifier(sid.as_raw()),
        }
    }
}
