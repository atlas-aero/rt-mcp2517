use crate::status::{OperationMode, OperationStatus};

#[test]
fn test_operation_status_from_register() {
    assert_eq!(
        OperationMode::NormalCANFD,
        OperationStatus::from_register(0b0001_0100).mode
    );
    assert_eq!(OperationMode::Sleep, OperationStatus::from_register(0b0011_0100).mode);
    assert_eq!(
        OperationMode::InternalLoopback,
        OperationStatus::from_register(0b0101_0100).mode
    );
    assert_eq!(
        OperationMode::ListenOnly,
        OperationStatus::from_register(0b0111_0100).mode
    );
    assert_eq!(
        OperationMode::Configuration,
        OperationStatus::from_register(0b1001_0100).mode
    );
    assert_eq!(
        OperationMode::ExternalLoopback,
        OperationStatus::from_register(0b1011_0100).mode
    );
    assert_eq!(
        OperationMode::NormalCAN2_0,
        OperationStatus::from_register(0b1101_0100).mode
    );
    assert_eq!(
        OperationMode::RestrictedOperation,
        OperationStatus::from_register(0b1111_0100).mode
    );

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
