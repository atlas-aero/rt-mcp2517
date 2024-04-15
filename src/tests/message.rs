use crate::message::{DLCError, TxMessage, DLC};
use embedded_can::Id;
use embedded_can::{ExtendedId, StandardId};

const EXTENDED_ID: u32 = 0x14C92A2B;
const STANDARD_ID: u16 = 0x6A5;

#[test]
fn test_extended_id() {
    let mut data = [0u8; 8];
    let extended_id = ExtendedId::new(EXTENDED_ID).unwrap();
    let message = TxMessage::new(Id::Extended(extended_id), &mut data, false, false).unwrap();
    assert!(message.header.identifier_extension_flag());
    assert_eq!(message.header.extended_identifier(), 0b01_0010_1010_0010_1011);
    assert_eq!(message.header.standard_identifier(), 0b101_0011_0010);
}
#[test]
fn test_standard_id() {
    let mut data = [0u8; 8];
    let standard_id = StandardId::new(STANDARD_ID).unwrap();
    let message = TxMessage::new(Id::Standard(standard_id), &mut data, false, false).unwrap();
    assert!(!message.header.identifier_extension_flag());
    assert_eq!(message.header.extended_identifier(), 0b00_0000_0000_0000_0000);
    assert_eq!(message.header.standard_identifier(), 0b110_1010_0101);
}
#[test]
fn test_dlc_success() {
    let mut data = [0u8; 13];
    let standard_id = StandardId::new(STANDARD_ID).unwrap();
    let message = TxMessage::new(Id::Standard(standard_id), &mut data, true, false).unwrap();
    assert_eq!(message.header.data_length_code(), DLC::Sixteen);
}

#[test]
fn test_dlc_error() {
    let mut data_2_0 = [0u8; 10];
    let mut data_fd = [0u8; 65];
    let standard_id = StandardId::new(STANDARD_ID).unwrap();
    let message_2_0 = TxMessage::new(Id::Standard(standard_id), &mut data_2_0, false, false);
    let message_fd = TxMessage::new(Id::Standard(standard_id), &mut data_fd, true, false);
    assert_eq!(message_2_0.unwrap_err(), DLCError::InvalidLength(10));
    assert_eq!(message_fd.unwrap_err(), DLCError::InvalidLength(65));
}
