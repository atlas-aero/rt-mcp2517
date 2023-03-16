use crate::status::OperationMode::NormalCANFD;
use crate::status::{OperationMode, OperationStatus, OscillatorStatus};
use OperationMode::{
    Configuration, ExternalLoopback, InternalLoopback, ListenOnly, NormalCAN2_0, RestrictedOperation, Sleep,
};

#[test]
fn test_operation_status_from_register() {
    assert_eq!(NormalCANFD, OperationStatus::from_register(0b0001_0100).mode);
    assert_eq!(Sleep, OperationStatus::from_register(0b0011_0100).mode);
    assert_eq!(InternalLoopback, OperationStatus::from_register(0b0101_0100).mode);
    assert_eq!(ListenOnly, OperationStatus::from_register(0b0111_0100).mode);
    assert_eq!(Configuration, OperationStatus::from_register(0b1001_0100).mode);
    assert_eq!(ExternalLoopback, OperationStatus::from_register(0b1011_0100).mode);
    assert_eq!(NormalCAN2_0, OperationStatus::from_register(0b1101_0100).mode);
    assert_eq!(RestrictedOperation, OperationStatus::from_register(0b1111_0100).mode);

    assert!(OperationStatus::from_register(0b0001_0100).txq_reserved);
    assert!(!OperationStatus::from_register(0b0000_0100).txq_reserved);

    assert!(OperationStatus::from_register(0b0001_1100).store_transmit_event);
    assert!(!OperationStatus::from_register(0b0000_0100).store_transmit_event);

    assert!(OperationStatus::from_register(0b0001_1110).error_trans_listen_only_mode);
    assert!(!OperationStatus::from_register(0b0000_0000).error_trans_listen_only_mode);

    assert!(OperationStatus::from_register(0b0001_1110).transmit_esi_gateway);
    assert!(!OperationStatus::from_register(0b0000_0100).transmit_esi_gateway);

    assert!(OperationStatus::from_register(0b0001_1111).restrict_retransmission);
    assert!(!OperationStatus::from_register(0b0000_0100).restrict_retransmission);
}

#[test]
fn test_oscillator_status_from_register() {
    assert!(OscillatorStatus::from_register(0b0001_0100).sclk_ready);
    assert!(!OscillatorStatus::from_register(0b0000_0100).sclk_ready);

    assert!(OscillatorStatus::from_register(0b0001_0100).clock_ready);
    assert!(!OscillatorStatus::from_register(0b0000_0000).clock_ready);

    assert!(OscillatorStatus::from_register(0b0001_0101).pll_ready);
    assert!(!OscillatorStatus::from_register(0b0000_0100).pll_ready);
}
