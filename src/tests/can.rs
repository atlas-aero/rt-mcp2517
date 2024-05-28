use crate::can::{BusError, ConfigError, Controller};
use crate::config::{
    ClockConfiguration, ClockOutputDivisor, Configuration, FifoConfiguration, PLLSetting, PayloadSize, RequestMode,
    RetransmissionAttempts, SystemClockDivisor,
};
use crate::filter::Filter;
use crate::message::{Can20, CanFd, TxMessage,RxHeader};
use crate::mocks::{MockPin, MockSPIBus, TestClock};
use crate::status::OperationMode;
use alloc::vec;
use bytes::Bytes;
use mockall::Sequence;
use embedded_can::{ExtendedId, Id, StandardId};

#[test]
fn test_configure_correct() {
    let clock = TestClock::new(vec![
        100,    // Config mode: Timer start,
        200,    // Config mode: First expiration check
        300,    // Config mode: Second expiration check
        10_000, // Request mode: Timer start
        10_100, // Request mode: First expiration check
    ]);

    let mut bus = MockSPIBus::new();
    // Request configuration mode
    bus.expect_transfer().times(1).returning(move |data| {
        assert_eq!([0x20, 0x3, 0b0000_1100], data);
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

    // Writing clock configuration
    bus.expect_transfer().times(1).returning(move |data| {
        assert_eq!([0x2E, 0x0, 0b0110_0001], data);
        Ok(&[0x0, 0x0, 0x0])
    });

    // Writing RX FIFO configuration
    bus.expect_transfer().times(1).returning(move |data| {
        assert_eq!([0x20, 0x5F, 0b0000_1111], data);
        Ok(&[0x0, 0x0, 0x0])
    });

    // Writing TX FIFO configuration
    bus.expect_transfer().times(1).returning(move |data| {
        assert_eq!([0x20, 0x6A, 0b0010_1010], data);
        Ok(&[0x0, 0x0, 0x0])
    });

    // Writing TX FIFO configuration
    bus.expect_transfer().times(1).returning(move |data| {
        assert_eq!([0x20, 0x6B, 0b0001_0011], data);
        Ok(&[0x0, 0x0, 0x0])
    });

    // Writing TX FIFO configuration
    bus.expect_transfer().times(1).returning(move |data| {
        assert_eq!([0x20, 0x68, 0b1000_0000], data);
        Ok(&[0x0, 0x0, 0x0])
    });

    // Enable filter for RX Fifo
    // filter disable
    bus.expect_transfer().times(1).returning(move |data| {
        assert_eq!([0x21, 0xD0, 0x00], data);
        Ok(&[0u8, 0u8, 0u8])
    });

    // write F02BP
    bus.expect_transfer().times(1).returning(move |data| {
        assert_eq!([0x21, 0xD0, 0x01], data);
        Ok(&[0u8, 0u8, 0u8])
    });

    // enable filter
    bus.expect_transfer().times(1).returning(move |data| {
        assert_eq!([0x21, 0xD0, 0b1000_0001], data);
        Ok(&[0u8, 0u8, 0u8])
    });

    // Request normal CAN 2.0B mode
    bus.expect_transfer().times(1).returning(move |data| {
        assert_eq!([0x20, 0x3, 0b0000_1110], data);
        Ok(&[0x0, 0x0, 0x0])
    });

    // Request mode reached
    bus.expect_transfer().times(1).returning(move |data| {
        assert_eq!([0x30, 0x2, 0x0], data);
        Ok(&[0x0, 0x0, 0b1100_0000])
    });

    let mut pin_cs = MockPin::new();
    pin_cs.expect_set_low().times(13).return_const(Ok(()));
    pin_cs.expect_set_high().times(13).return_const(Ok(()));

    let mut controller = Controller::new(bus, pin_cs);
    controller
        .configure(
            &Configuration {
                clock: ClockConfiguration {
                    clock_output: ClockOutputDivisor::DivideBy10,
                    system_clock: SystemClockDivisor::DivideBy1,
                    disable_clock: false,
                    pll: PLLSetting::TenTimesPLL,
                },
                fifo: FifoConfiguration {
                    rx_size: 16,
                    tx_attempts: RetransmissionAttempts::Three,
                    tx_priority: 10,
                    pl_size: PayloadSize::EightBytes,
                    tx_size: 20,
                    tx_enable: true,
                },
                mode: RequestMode::NormalCAN2_0,
            },
            &clock,
        )
        .unwrap();
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
    assert_eq!(
        ConfigError::ConfigurationModeTimeout,
        controller.configure(&Configuration::default(), &clock).unwrap_err()
    );
}

const EXTENDED_ID: u32 = 0x14C92A2B; //0b000(1_0100_1100_10)(01_0010_1010_0010_1011)
const STANDARD_ID: u16 = 0x6A5;

#[test]
fn test_transmit_can20() {
    let mut mocks = Mocks::default();
    let mut seq = Sequence::new();
    let payload: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    let payload_bytes = Bytes::copy_from_slice(&payload);

    let msg_type = Can20 {};

    let identifier = ExtendedId::new(EXTENDED_ID).unwrap();
    let tx_message = TxMessage::new(msg_type, payload_bytes, Id::Extended(identifier)).unwrap();
    let tx_message_copy = tx_message.clone();

    // mock fifo status register read byte 0 (1st attempt) -> tx fifo full
    mocks.mock_register_read::<0b0000_0000>([0x30, 0x6C], &mut seq);

    // mock fifo status register read byte 0 (2nd attempt) -> tx fifo not full
    mocks.mock_register_read::<0b0000_0001>([0x30, 0x6C], &mut seq);

    // mock read operation status
    mocks.mock_register_read::<0b1100_0000>([0x30, 0x2], &mut seq);

    // mock fifo user address register read (reading 32 bits) --> address = 0x4A2
    mocks.mock_read32::<0x00_00_04_A2>([0x30, 0x70], &mut seq);

    // mock writing message in RAM specified by fifo user address (0x4A2)
    // transfer cmd+tx_header

    mocks
        .pin_cs
        .expect_set_low()
        .times(1)
        .return_const(Ok(()))
        .in_sequence(&mut seq);

    mocks
        .bus
        .expect_transfer()
        .times(1)
        .returning(move |data| {
            let mut cmd_and_header_buffer = [0u8; 10];
            cmd_and_header_buffer[0] = 0x24;
            cmd_and_header_buffer[1] = 0xA2;
            cmd_and_header_buffer[2..].copy_from_slice(&tx_message.header.into_bytes());

            assert_eq!(cmd_and_header_buffer, data);
            Ok(&[0u8; 10])
        })
        .in_sequence(&mut seq);

    // transfer payload
    mocks
        .bus
        .expect_transfer()
        .times(1)
        .returning(move |data| {
            assert_eq!(payload, data);
            Ok(&[0u8; 8])
        })
        .in_sequence(&mut seq);

    mocks
        .pin_cs
        .expect_set_high()
        .times(1)
        .return_const(Ok(()))
        .in_sequence(&mut seq);

    // mock setting of bits txreq and uinc
    mocks
        .pin_cs
        .expect_set_low()
        .times(1)
        .return_const(Ok(()))
        .in_sequence(&mut seq);

    mocks
        .bus
        .expect_transfer()
        .times(1)
        .returning(move |data| {
            assert_eq!([0x20, 0x69, 0x03], data);
            Ok(&[0u8; 3])
        })
        .in_sequence(&mut seq);

    mocks
        .pin_cs
        .expect_set_high()
        .times(1)
        .return_const(Ok(()))
        .in_sequence(&mut seq);

    // mock reading of fifo control register
    // 1st attempt -> txreq still set ->not all messages inside tx fifo have been transmitted
    mocks.mock_register_read::<0x02>([0x30, 0x69], &mut seq);
    // 2nd attempt -> txreq cleared -> all messages inside tx fifo have been transmitted
    mocks.mock_register_read::<0x00>([0x30, 0x69], &mut seq);

    let result = mocks.into_controller().transmit(&tx_message_copy);
    assert!(result.is_ok());
}

#[test]
fn test_transmit_can_fd() {
    let mut mocks = Mocks::default();
    let mut seq = Sequence::new();
    let payload = [1u8; 64];
    let payload_bytes = Bytes::copy_from_slice(&payload);

    let msg_type = CanFd { bitrate_switch: false };

    let identifier = ExtendedId::new(EXTENDED_ID).unwrap();
    let tx_message = TxMessage::new(msg_type, payload_bytes, Id::Extended(identifier)).unwrap();
    let tx_message_copy = tx_message.clone();

    // mock fifo status register read byte 0 (1st attempt) -> tx fifo full
    mocks.mock_register_read::<0b0000_0000>([0x30, 0x6C], &mut seq);

    // mock fifo status register read byte 0 (2nd attempt) -> tx fifo not full
    mocks.mock_register_read::<0b0000_0001>([0x30, 0x6C], &mut seq);

    // mock read operation status
    mocks.mock_register_read::<0b0000_0000>([0x30, 0x2], &mut seq);

    // mock fifo user address register read (reading 32 bits) --> address = 0x4A2
    mocks.mock_read32::<0x00_00_04_A2>([0x30, 0x70], &mut seq);

    // mock writing message in RAM specified by fifo user address (0x4A2)
    // transfer cmd+tx_header
    mocks
        .pin_cs
        .expect_set_low()
        .times(1)
        .return_const(Ok(()))
        .in_sequence(&mut seq);

    mocks
        .bus
        .expect_transfer()
        .times(1)
        .returning(move |data| {
            let mut cmd_and_header_buffer = [0u8; 10];
            cmd_and_header_buffer[0] = 0x24;
            cmd_and_header_buffer[1] = 0xA2;
            cmd_and_header_buffer[2..].copy_from_slice(&tx_message.header.into_bytes());

            assert_eq!(cmd_and_header_buffer, data);
            Ok(&[0u8; 10])
        })
        .in_sequence(&mut seq);

    // transfer payload
    mocks
        .bus
        .expect_transfer()
        .times(1)
        .returning(move |data| {
            assert_eq!(payload, data);
            Ok(&[1u8; 64])
        })
        .in_sequence(&mut seq);

    mocks
        .pin_cs
        .expect_set_high()
        .times(1)
        .return_const(Ok(()))
        .in_sequence(&mut seq);

    // mock setting of bits txreq and uinc
    mocks
        .pin_cs
        .expect_set_low()
        .times(1)
        .return_const(Ok(()))
        .in_sequence(&mut seq);

    mocks
        .bus
        .expect_transfer()
        .times(1)
        .returning(move |data| {
            assert_eq!([0x20, 0x69, 0x03], data);
            Ok(&[0u8; 3])
        })
        .in_sequence(&mut seq);

    mocks
        .pin_cs
        .expect_set_high()
        .times(1)
        .return_const(Ok(()))
        .in_sequence(&mut seq);

    // mock reading of fifo control register
    // 1st attempt -> txreq still set ->not all messages inside tx fifo have been transmitted
    mocks.mock_register_read::<0x02>([0x30, 0x69], &mut seq);
    // 2nd attempt -> txreq cleared -> all messages inside tx fifo have been transmitted
    mocks.mock_register_read::<0x00>([0x30, 0x69], &mut seq);

    let result = mocks.into_controller().transmit(&tx_message_copy);
    assert!(result.is_ok());
}

#[test]
fn test_receive() {
    let mut mocks = Mocks::default();

    let id = ExtendedId::new(EXTENDED_ID).unwrap();

    // custom Rx message header for testing
    let message_header = RxHeader::new_test_cfg(Id::Extended(id));

    let mut message_buff = [0u8; 16];

    // status register read
    mocks.mock_register_read::<0b0000_0001>([0x30, 0x60]);

    // user address register read
    mocks.mock_read32::<0x00_00_04_7C>([0x30, 0x64]);

    // Message read from RAM address 0x47C
    //transfer cmd+address
    mocks.bus.expect_transfer().times(1).returning(move |data| {
        assert_eq!([0x34, 0x7C], data);
        Ok(&[0u8; 2])
    });

    // transfer message_buff where message bytes are placed
    mocks.bus.expect_transfer().times(1).returning(move |data| {
        assert_eq!([0u8; 16], data);
        Ok(&[0x09, 0x51, 0x5D, 0x32, 0u8, 0u8, 0u8, 0x18, 1, 2, 3, 4, 5, 6, 7, 8])
    });

    // set uinc bit in Rx FIFO control register
    mocks.bus.expect_transfer().times(1).returning(move |data| {
        assert_eq!([0x20, 0x5D, 0b0000_0001], data);
        Ok(&[0u8; 3])
    });

    mocks.pin_cs.expect_set_low().times(2).return_const(Ok(()));
    mocks.pin_cs.expect_set_high().times(2).return_const(Ok(()));

    let result = mocks.into_controller().receive(&mut message_buff).unwrap();

    assert_eq!(result[..8], message_header.into_bytes());
    assert_eq!(result[8..], [1, 2, 3, 4, 5, 6, 7, 8]);
}

#[test]
fn test_reset_command() {
    let mut mocks = Mocks::default();
    let mut seq = Sequence::new();

    mocks.bus.expect_transfer().times(1).returning(move |data| {
        assert_eq!([0x00, 0x00, 0x00], data);
        Ok(&[0u8; 3])
    });

    mocks
        .pin_cs
        .expect_set_low()
        .times(1)
        .return_const(Ok(()))
        .in_sequence(&mut seq);

    mocks
        .pin_cs
        .expect_set_high()
        .times(1)
        .return_const(Ok(()))
        .in_sequence(&mut seq);

    let result = mocks.into_controller().reset();
    assert!(result.is_ok());
}

#[test]
fn test_set_filter_object_standard_id() {
    let id_standard = StandardId::new(STANDARD_ID).unwrap();
    let mut filter = Filter::new(Id::Standard(id_standard), 1).unwrap();

    // mask 2 lsb of standard id -> MSID <1:0> should be set
    filter.set_mask_standard_id(0b000_0000_0011);

    // MIDE should be set and EXIDE should be cleared
    filter.match_standard_only();

    let mut mocks = Mocks::default();

    // disable filter 0
    mocks.bus.expect_transfer().times(1).returning(move |data| {
        assert_eq!([0x21, 0xD1, 0x00], data);
        Ok(&[0u8; 3])
    });

    // write filter value
    mocks.bus.expect_transfer().times(1).returning(move |data| {
        assert_eq!([0x21, 0xF8, 0xA5, 0x6, 0x0, 0x0], data);
        Ok(&[0u8; 2])
    });

    // write mask value
    mocks.bus.expect_transfer().times(1).returning(move |data| {
        assert_eq!([0x21, 0xFC, 0x3, 0u8, 0u8, 0x40], data);
        Ok(&[0u8; 6])
    });

    mocks.pin_cs.expect_set_low().times(3).return_const(Ok(()));
    mocks.pin_cs.expect_set_high().times(3).return_const(Ok(()));

    let result = mocks.into_controller().set_filter_object(filter);

    assert!(result.is_ok());
}
#[test]
fn test_set_filter_object_extended_id() {
    let id_extended = ExtendedId::new(EXTENDED_ID).unwrap();
    let mut filter = Filter::new(Id::Extended(id_extended), 0).unwrap();

    // mask the 2 msb of extended id -> MSID<10:9> should be set
    filter.set_mask_extended_id(0b1_1000_0000_0000_0000_0000_0000_0000);

    let mut mocks = Mocks::default();

    // disable filter 0
    mocks.bus.expect_transfer().times(1).returning(move |data| {
        assert_eq!([0x21, 0xD0, 0x00], data);
        Ok(&[0u8; 3])
    });

    // write filter value
    mocks.bus.expect_transfer().times(1).returning(move |data| {
        assert_eq!([0x21, 0xF0, 0x32, 0x5D, 0x51, 0x09], data);
        Ok(&[0u8; 2])
    });

    // write mask value
    mocks.bus.expect_transfer().times(1).returning(move |data| {
        assert_eq!([0x21, 0xF4, 0u8, 0x6, 0u8, 0u8], data);
        Ok(&[0u8; 6])
    });

    mocks.pin_cs.expect_set_low().times(3).return_const(Ok(()));
    mocks.pin_cs.expect_set_high().times(3).return_const(Ok(()));

    let result_extended = mocks.into_controller().set_filter_object(filter);

    assert!(result_extended.is_ok());
}

#[test]
fn test_request_mode_timeout() {
    let clock = TestClock::new(vec![
        100,    // Config mode: Timer start,
        200,    // Config mode: First expiration check
        300,    // Config mode: Second expiration check
        10_000, // Request mode: Timer start
        10_100, // Request mode: First expiration check
        15_000, // Request mode: Second expiration check (expired)
    ]);

    let mut bus = MockSPIBus::new();
    // Request configuration mode
    bus.expect_transfer().times(1).returning(move |_| Ok(&[0x0, 0x0, 0x0]));

    // Still in normal mode
    bus.expect_transfer().times(1).returning(move |_| Ok(&[0x0, 0x0, 0b0001_0100]));

    // Configuration mode
    bus.expect_transfer().times(1).returning(move |_| Ok(&[0x0, 0x0, 0b1001_0100]));

    // Writing configuration registers
    bus.expect_transfer().times(5).returning(move |_| Ok(&[0x0, 0x0, 0x0]));

    // Enable filter for RX Fifo
    // filter disable
    bus.expect_transfer().times(1).returning(move |data| {
        assert_eq!([0x21, 0xD0, 0x00], data);
        Ok(&[0u8, 0u8, 0u8])
    });

    // write F02BP
    bus.expect_transfer().times(1).returning(move |data| {
        assert_eq!([0x21, 0xD0, 0x01], data);
        Ok(&[0u8, 0u8, 0u8])
    });

    // enable filter
    bus.expect_transfer().times(1).returning(move |data| {
        assert_eq!([0x21, 0xD0, 0b1000_0001], data);
        Ok(&[0u8, 0u8, 0u8])
    });

    // Request normal CAN FD mode
    bus.expect_transfer().times(1).returning(move |data| {
        assert_eq!([0x20, 0x3, 0b0000_1000], data);
        Ok(&[0x0, 0x0, 0x0])
    });

    // Still configuration mode
    bus.expect_transfer().times(2).returning(move |data| {
        assert_eq!([0x30, 0x2, 0x0], data);
        Ok(&[0x0, 0x0, 0b1001_0100])
    });

    let mut pin_cs = MockPin::new();
    pin_cs.expect_set_low().times(14).return_const(Ok(()));
    pin_cs.expect_set_high().times(14).return_const(Ok(()));

    let mut controller = Controller::new(bus, pin_cs);
    assert_eq!(
        ConfigError::RequestModeTimeout,
        controller.configure(&Configuration::default(), &clock).unwrap_err()
    );
}

#[test]
fn test_configure_cs_pin_error() {
    let clock = TestClock::new(vec![]);
    let mut mocks = Mocks::default();
    mocks.mock_cs_error();

    assert_eq!(
        ConfigError::BusError(BusError::CSError(21)),
        mocks
            .into_controller()
            .configure(&Configuration::default(), &clock)
            .unwrap_err()
    );
}

#[test]
fn test_configure_transfer_error() {
    let clock = TestClock::new(vec![]);
    let mut mocks = Mocks::default();
    mocks.mock_transfer_error();

    assert_eq!(
        ConfigError::BusError(BusError::TransferError(55)),
        mocks
            .into_controller()
            .configure(&Configuration::default(), &clock)
            .unwrap_err()
    );
}

#[test]
fn test_read_operation_status_correct() {
    let mut mocks = Mocks::default();
    let mut seq = Sequence::new();

    mocks.mock_register_read::<0b0001_0100>([0x30, 0x2], &mut seq);

    let status = mocks.into_controller().read_operation_status().unwrap();

    assert_eq!(OperationMode::NormalCANFD, status.mode);
    assert!(status.txq_reserved);
    assert!(!status.store_transmit_event);
    assert!(status.error_trans_listen_only_mode);
    assert!(!status.transmit_esi_gateway);
    assert!(!status.restrict_retransmission);
}

#[test]
fn test_read_operation_status_cs_error() {
    let mut mocks = Mocks::default();
    mocks.mock_cs_error();

    assert_eq!(
        BusError::CSError(21),
        mocks.into_controller().read_operation_status().unwrap_err()
    );
}

#[test]
fn test_read_operation_status_transfer_error() {
    let mut mocks = Mocks::default();
    mocks.mock_transfer_error();

    assert_eq!(
        BusError::TransferError(55),
        mocks.into_controller().read_operation_status().unwrap_err()
    );
}

#[test]
fn test_read_oscillator_status_correct() {
    let mut mocks = Mocks::default();
    let mut seq = Sequence::new();

    mocks.mock_register_read::<0b0001_0100>([0x3E, 0x1], &mut seq);

    let status = mocks.into_controller().read_oscillator_status().unwrap();

    assert!(status.sclk_ready);
    assert!(status.clock_ready);
    assert!(!status.pll_ready);
}

#[test]
fn test_read_oscillator_status_cs_error() {
    let mut mocks = Mocks::default();
    mocks.mock_cs_error();

    assert_eq!(
        BusError::CSError(21),
        mocks.into_controller().read_oscillator_status().unwrap_err()
    );
}

#[test]
fn test_read_oscillator_transfer_error() {
    let mut mocks = Mocks::default();
    mocks.mock_transfer_error();

    assert_eq!(
        BusError::TransferError(55),
        mocks.into_controller().read_oscillator_status().unwrap_err()
    );
}

#[test]
fn test_read_clock_configuration_correct() {
    let mut mocks = Mocks::default();
    let mut seq = Sequence::new();

    mocks.mock_register_read::<0b0110_0000>([0x3E, 0x0], &mut seq);

    let status = mocks.into_controller().read_clock_configuration().unwrap();

    assert_eq!(ClockOutputDivisor::DivideBy10, status.clock_output);
    assert_eq!(SystemClockDivisor::DivideBy1, status.system_clock);
    assert!(!status.disable_clock);
    assert_eq!(PLLSetting::DirectXTALOscillator, status.pll);
}

#[test]
fn test_read_clock_configuration_cs_error() {
    let mut mocks = Mocks::default();
    mocks.mock_cs_error();

    assert_eq!(
        BusError::CSError(21),
        mocks.into_controller().read_clock_configuration().unwrap_err()
    );
}

#[test]
fn test_read_clock_configuration_transfer_error() {
    let mut mocks = Mocks::default();
    mocks.mock_transfer_error();

    assert_eq!(
        BusError::TransferError(55),
        mocks.into_controller().read_clock_configuration().unwrap_err()
    );
}

#[derive(Default)]
struct Mocks {
    bus: MockSPIBus,
    pin_cs: MockPin,
}

impl Mocks {
    pub fn into_controller(self) -> Controller<MockSPIBus, MockPin, TestClock> {
        Controller::new(self.bus, self.pin_cs)
    }

    /// Simulates a SPI transfer fault
    pub fn mock_transfer_error(&mut self) {
        self.bus.expect_transfer().times(1).return_const(Err(55));
        self.pin_cs.expect_set_low().times(1).return_const(Ok(()));
        self.pin_cs.expect_set_high().times(1).return_const(Ok(()));
    }

    /// Simulates a CS pin set error
    pub fn mock_cs_error(&mut self) {
        self.pin_cs.expect_set_low().times(1).return_const(Err(21));
    }

    /// Mocks the reading of a single register byte
    pub fn mock_register_read<const REG: u8>(&mut self, expected_command: [u8; 2], seq: &mut Sequence) {
        let expected_buffer = [expected_command[0], expected_command[1], 0x0];

        self.pin_cs.expect_set_low().times(1).return_const(Ok(())).in_sequence(seq);

        self.bus
            .expect_transfer()
            .times(1)
            .returning(move |data| {
                assert_eq!(expected_buffer, data);
                Ok(&[0x0, 0x0, REG])
            })
            .in_sequence(seq);

        self.pin_cs.expect_set_high().times(1).return_const(Ok(())).in_sequence(seq);
    }

    /// mocks 4-byte register read
    pub fn mock_read32<const REG: u32>(&mut self, expected_command: [u8; 2], seq: &mut Sequence) {
        let expected_buffer = [expected_command[0], expected_command[1], 0u8, 0u8, 0u8, 0u8];

        self.pin_cs.expect_set_low().times(1).return_const(Ok(())).in_sequence(seq);

        self.bus
            .expect_transfer()
            .times(1)
            .returning(move |data| {
                assert_eq!(expected_buffer, data);
                Ok(&[
                    0x0,
                    0x0,
                    REG as u8,
                    (REG >> 8) as u8,
                    (REG >> 16) as u8,
                    (REG >> 24) as u8,
                ])
            })
            .in_sequence(seq);

        self.pin_cs.expect_set_high().times(1).return_const(Ok(())).in_sequence(seq);
    }
}
