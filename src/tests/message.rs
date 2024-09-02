use crate::message::{Can20, CanFd, DLCError, TxMessage, DLC};
use bytes::Bytes;
use embedded_can::Id;
use embedded_can::{ExtendedId, StandardId};

const EXTENDED_ID: u32 = 0x14C92A2B;

const STANDARD_ID: u16 = 0x6A5;

#[test]
fn test_extended_id() {
    let payload_bytes = Bytes::copy_from_slice(&[0u8; 8]);
    let extended_id = ExtendedId::new(EXTENDED_ID).unwrap();

    let msg_type = Can20 {};

    let message = TxMessage::new(msg_type, payload_bytes, Id::Extended(extended_id)).unwrap();

    assert!(message.header.identifier_extension_flag());
    assert_eq!(message.header.extended_identifier(), 0b01_0010_1010_0010_1011);
    assert_eq!(message.header.standard_identifier(), 0b101_0011_0010);
}

#[test]
fn test_standard_id() {
    let payload_bytes = Bytes::copy_from_slice(&[0u8; 8]);
    let standard_id = StandardId::new(STANDARD_ID).unwrap();

    let msg_type = Can20 {};

    let message = TxMessage::new(msg_type, payload_bytes, Id::Standard(standard_id)).unwrap();

    assert!(!message.header.identifier_extension_flag());
    assert_eq!(message.header.extended_identifier(), 0b00_0000_0000_0000_0000);
    assert_eq!(message.header.standard_identifier(), 0b110_1010_0101);
}

#[test]
fn test_dlc_success() {
    let payload_bytes = Bytes::copy_from_slice(&[0u8; 13]);
    let standard_id = StandardId::new(STANDARD_ID).unwrap();

    let msg_type = CanFd { bitrate_switch: false };

    let message = TxMessage::new(msg_type, payload_bytes, Id::Standard(standard_id)).unwrap();

    assert_eq!(message.header.data_length_code(), DLC::Sixteen);
    assert!(message.header.fd_frame());

    let header_bytes = message.header.into_bytes();

    assert_eq!(header_bytes[7], 0b1000_1010);
}

#[test]
fn test_dlc_error() {
    let data_2_0 = [0u8; 10];
    let data_fd = [0u8; 65];

    let payload_bytes_2_0 = Bytes::copy_from_slice(&data_2_0);
    let payload_bytes_fd = Bytes::copy_from_slice(&data_fd);

    let can_msg_20 = Can20 {};
    let can_msg_fd = CanFd { bitrate_switch: false };

    let standard_id = StandardId::new(STANDARD_ID).unwrap();

    let message_2_0 = TxMessage::new(can_msg_20, payload_bytes_2_0, Id::Standard(standard_id));
    let message_fd = TxMessage::new(can_msg_fd, payload_bytes_fd, Id::Standard(standard_id));

    assert_eq!(message_2_0.unwrap_err(), DLCError::InvalidLength(10));
    assert_eq!(message_fd.unwrap_err(), DLCError::InvalidLength(65));
}
