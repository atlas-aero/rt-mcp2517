use crate::can::{BusError, ConfigError, Controller};
use crate::mocks::{MockPin, MockSPIBus, TestClock};
use crate::status::OperationMode;
use alloc::vec;

#[test]
fn test_configure_correct() {
    let clock = TestClock::new(vec![
        100, // Timer start,
        200, // First expiration check
        300, // Second expiration check
    ]);

    let mut bus = MockSPIBus::new();
    bus.expect_transfer().times(1).returning(move |data| {
        assert_eq!([0x20, 0x3, 0xC], data);
        Ok(&[0x0, 0x0, 0x0])
    });

    // Still in normal mode
    bus.expect_transfer().times(1).returning(move |data| {
        assert_eq!([0x30, 0x2, 0x0], data);
        Ok(&[0x0, 0x0, 0b0001_0100])
    });

    // Configuration mode
    bus.expect_transfer().times(1).returning(move |data| {
        assert_eq!([0x30, 0x2, 0x0], data);
        Ok(&[0x0, 0x0, 0b1001_0100])
    });

    let mut pin_cs = MockPin::new();
    pin_cs.expect_set_low().times(3).return_const(Ok(()));
    pin_cs.expect_set_high().times(3).return_const(Ok(()));

    let mut controller = Controller::new(bus, pin_cs);
    controller.configure(&clock).unwrap();
}

#[test]
fn test_configure_mode_timeout() {
    let clock = TestClock::new(vec![
        100,  // Timer start,
        200,  // First expiration check
        2500, // Second expiration check
    ]);

    let mut bus = MockSPIBus::new();
    bus.expect_transfer().times(1).returning(move |data| {
        assert_eq!([0x20, 0x3, 0xC], data);
        Ok(&[0x0, 0x0, 0x0])
    });

    // Still in normal mode
    bus.expect_transfer().times(1).returning(move |data| {
        assert_eq!([0x30, 0x2, 0x0], data);
        Ok(&[0x0, 0x0, 0b0001_0100])
    });

    // Configuration mode
    bus.expect_transfer().times(1).returning(move |data| {
        assert_eq!([0x30, 0x2, 0x0], data);
        Ok(&[0x0, 0x0, 0b0001_0100])
    });

    let mut pin_cs = MockPin::new();
    pin_cs.expect_set_low().times(3).return_const(Ok(()));
    pin_cs.expect_set_high().times(3).return_const(Ok(()));

    let mut controller = Controller::new(bus, pin_cs);
    assert_eq!(ConfigError::ModeTimeout, controller.configure(&clock).unwrap_err());
}

#[test]
fn test_configure_cs_pin_error() {
    let clock = TestClock::new(vec![]);
    let bus = MockSPIBus::new();
    let mut pin_cs = MockPin::new();
    pin_cs.expect_set_low().times(1).return_const(Err(20));

    let mut controller = Controller::new(bus, pin_cs);
    assert_eq!(
        ConfigError::BusError(BusError::CSError(20)),
        controller.configure(&clock).unwrap_err()
    );
}

#[test]
fn test_configure_transfer_error() {
    let clock = TestClock::new(vec![]);

    let mut bus = MockSPIBus::new();
    bus.expect_transfer().times(1).returning(move |data| {
        assert_eq!([0x20, 0x3, 0xC], data);
        Err(50)
    });

    let mut pin_cs = MockPin::new();
    pin_cs.expect_set_low().times(1).return_const(Ok(()));
    pin_cs.expect_set_high().times(1).return_const(Ok(()));

    let mut controller = Controller::new(bus, pin_cs);
    assert_eq!(
        ConfigError::BusError(BusError::TransferError(50)),
        controller.configure(&clock).unwrap_err()
    );
}

#[test]
fn test_read_operation_status_correct() {
    let mut bus = MockSPIBus::new();

    bus.expect_transfer().times(1).returning(move |data| {
        assert_eq!([0x30, 0x2, 0x0], data);
        Ok(&[0x0, 0x0, 0b0001_0100])
    });

    let mut pin_cs = MockPin::new();
    pin_cs.expect_set_low().times(1).return_const(Ok(()));
    pin_cs.expect_set_high().times(1).return_const(Ok(()));

    let mut controller: Controller<_, _, TestClock> = Controller::new(bus, pin_cs);
    let status = controller.read_operation_status().unwrap();

    assert_eq!(OperationMode::NormalCANFD, status.mode);
    assert!(status.txq_reserved);
    assert!(!status.store_transmit_event);
    assert!(status.error_trans_listen_only_mode);
    assert!(!status.transmit_esi_gateway);
    assert!(!status.restrict_retransmission);
}

#[test]
fn test_read_operation_status_cs_error() {
    let bus = MockSPIBus::new();
    let mut pin_cs = MockPin::new();
    pin_cs.expect_set_low().times(1).return_const(Err(21));

    let mut controller: Controller<_, _, TestClock> = Controller::new(bus, pin_cs);
    assert_eq!(BusError::CSError(21), controller.read_operation_status().unwrap_err());
}

#[test]
fn test_read_operation_status_transfer_error() {
    let mut bus = MockSPIBus::new();
    bus.expect_transfer().times(1).returning(move |data| {
        assert_eq!([0x30, 0x2, 0x0], data);
        Err(55)
    });

    let mut pin_cs = MockPin::new();
    pin_cs.expect_set_low().times(1).return_const(Ok(()));
    pin_cs.expect_set_high().times(1).return_const(Ok(()));

    let mut controller: Controller<_, _, TestClock> = Controller::new(bus, pin_cs);
    assert_eq!(
        BusError::TransferError(55),
        controller.read_operation_status().unwrap_err()
    );
}
