use crate::can::CanController;
use crate::can::{CanError, MCP2517};
use crate::config::{
    BitRateConfig, CanBaudRate, ClockConfiguration, ClockOutputDivisor, Configuration, FifoConfiguration, PLLSetting,
    PayloadSize, RequestMode, RetransmissionAttempts, SysClk, SystemClockDivisor,
};
use crate::example::{ExampleClock, ExampleSPIDevice};
use crate::filter::Filter;
use crate::message::{Can20, CanFd, TxMessage};
use crate::mocks::{MockSPIDevice, SPIError, TestClock};
use crate::status::OperationMode;
use alloc::vec;
use byteorder::{BigEndian, ByteOrder, LittleEndian};
use bytes::Bytes;
use embedded_can::{ExtendedId, Id, StandardId};
use embedded_hal::spi::Operation;
use mockall::Sequence;

/// CAN configuration mock
fn expect_config(spi_dev: &mut Mocks, seq: &mut Sequence) {
    // Writing clock configuration
    spi_dev.expect_register_write([0x2E, 0x0, 0b0110_0001], seq);

    // Writing NBT configuration register
    spi_dev.mock_write32([0x20, 0x04, 1, 15, 62, 0], seq);

    // Writing RX FIFO configuration
    spi_dev.expect_register_write([0x20, 0x5F, 0b0000_1111], seq);

    // Writing TX FIFO configuration
    spi_dev.expect_register_write([0x20, 0x6A, 0b0010_1010], seq);

    // Writing TX FIFO configuration
    spi_dev.expect_register_write([0x20, 0x6B, 0b0001_0011], seq);

    // Writing TX FIFO configuration
    spi_dev.expect_register_write([0x20, 0x68, 0b1000_0000], seq);

    // Enable filter for RX Fifo
    // filter disable
    spi_dev.expect_register_write([0x21, 0xD0, 0x00], seq);

    // write F02BP
    spi_dev.expect_register_write([0x21, 0xD0, 0x01], seq);

    // enable filter
    spi_dev.expect_register_write([0x21, 0xD0, 0b1000_0001], seq);
}

#[test]
fn test_configure_correct() {
    let clock = TestClock::new(vec![
        100,    // Config mode: Timer start,
        200,    // Config mode: First expiration check
        300,    // Config mode: Second expiration check
        10_000, // Request mode: Timer start
        10_100, // Request mode: First expiration check
    ]);

    let mut mock = Mocks::new();
    let mut sequence = Sequence::new();

    // Request configuration mode
    mock.expect_register_write([0x20, 0x3, 0b0000_1100], &mut sequence);

    // Still in normal mode
    mock.mock_register_read::<0b0001_0100>([0x30, 0x2], &mut sequence);

    // Configuration mode
    mock.mock_register_read::<0b1001_0100>([0x30, 0x2], &mut sequence);

    expect_config(&mut mock, &mut sequence);

    // Request normal CAN 2.0B mode
    mock.expect_register_write([0x20, 0x3, 0b0000_1110], &mut sequence);

    // Request mode reached
    mock.mock_register_read::<0b1100_0000>([0x30, 0x2], &mut sequence);

    mock.into_controller()
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
                bit_rate: BitRateConfig::default(),
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
    let mut seq = Sequence::new();

    let mut mock = Mocks::new();
    mock.expect_register_write([0x20, 0x3, 0xC], &mut seq);

    // Still in normal mode
    mock.mock_register_read::<0b0001_0100>([0x30, 0x2], &mut seq);

    // Configuration mode
    mock.mock_register_read::<0b0001_0100>([0x30, 0x2], &mut seq);

    let res = mock.into_controller().configure(&Configuration::default(), &clock);

    assert_eq!(CanError::ConfigurationModeTimeout, res.unwrap_err());
}

const EXTENDED_ID: u32 = 0x14C92A2B; //0b000(1_0100_1100_10)(01_0010_1010_0010_1011)
const STANDARD_ID: u16 = 0x6A5;

#[test]
fn test_transmit_can20() {
    let mut mocks = Mocks::default();
    let mut seq = Sequence::new();
    let payload: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    let payload_bytes = Bytes::copy_from_slice(&payload);

    let msg_type = Can20::<8> {};

    let identifier = ExtendedId::new(EXTENDED_ID).unwrap();
    let tx_message = TxMessage::new(msg_type, payload_bytes, Id::Extended(identifier)).unwrap();
    let tx_message_copy = tx_message.clone();

    // mock fifo status register read byte 0 (1st attempt) -> TX fifo full
    mocks.mock_register_read::<0b0000_0000>([0x30, 0x6C], &mut seq);

    // mock fifo status register read byte 0 (2nd attempt) -> TX fifo not full
    mocks.mock_register_read::<0b0000_0001>([0x30, 0x6C], &mut seq);

    // mock read operation status
    mocks.mock_register_read::<0b1100_0000>([0x30, 0x2], &mut seq);

    // mock fifo user address register read (reading 32 bits) --> address = 0x4A2
    mocks.mock_read32::<0x00_00_04_A2>([0x30, 0x70], &mut seq);

    // mock writing message in RAM specified by fifo user address (0x4A2)
    // transfer cmd+tx_header
    let mut cmd_and_header_buffer = [0u8; 10];
    cmd_and_header_buffer[0] = 0x28;
    cmd_and_header_buffer[1] = 0xA2;

    cmd_and_header_buffer[2..].copy_from_slice(&tx_message.header.into_bytes());

    for chunk in cmd_and_header_buffer[2..].chunks_exact_mut(4) {
        let num = BigEndian::read_u32(chunk);
        LittleEndian::write_u32(chunk, num);
    }

    mocks.expect_fifo_write_transaction(cmd_and_header_buffer, payload, &mut seq);

    mocks.expect_register_write([0x20, 0x69, 0x03], &mut seq);

    // mock reading of fifo control register
    // 1st attempt -> txreq still set ->not all messages inside tx fifo have been transmitted
    mocks.mock_register_read::<0x02>([0x30, 0x69], &mut seq);
    // 2nd attempt -> txreq cleared -> all messages inside tx fifo have been transmitted
    mocks.mock_register_read::<0x00>([0x30, 0x69], &mut seq);

    mocks.into_controller().transmit(&tx_message_copy, true).unwrap();
}

#[test]
fn test_transmit_can20_3_bytes() {
    let mut mocks = Mocks::default();
    let mut seq = Sequence::new();
    let payload: [u8; 3] = [1, 2, 3];
    let payload_bytes = Bytes::copy_from_slice(&payload);

    let msg_type = Can20::<4> {};

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
    let mut cmd_and_header_buffer = [0u8; 10];
    cmd_and_header_buffer[0] = 0x28;
    cmd_and_header_buffer[1] = 0xA2;

    cmd_and_header_buffer[2..].copy_from_slice(&tx_message.header.into_bytes());

    for chunk in cmd_and_header_buffer[2..].chunks_exact_mut(4) {
        let num = BigEndian::read_u32(chunk);
        LittleEndian::write_u32(chunk, num);
    }

    mocks.expect_fifo_write_transaction(cmd_and_header_buffer, payload, &mut seq);

    mocks.expect_register_write([0x20, 0x69, 0x03], &mut seq);

    // mock reading of fifo control register
    // 1st attempt -> txreq still set ->not all messages inside tx fifo have been transmitted
    mocks.mock_register_read::<0x02>([0x30, 0x69], &mut seq);
    // 2nd attempt -> txreq cleared -> all messages inside tx fifo have been transmitted
    mocks.mock_register_read::<0x00>([0x30, 0x69], &mut seq);

    mocks.into_controller().transmit(&tx_message_copy, true).unwrap();
}

#[test]
fn test_transmit_can_fd() {
    let mut mocks = Mocks::default();
    let mut seq = Sequence::new();
    let payload = [1u8; 64];
    let payload_bytes = Bytes::copy_from_slice(&payload);

    let msg_type = CanFd::<64> { bitrate_switch: false };

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

    let mut cmd_and_header_buffer = [0u8; 10];
    cmd_and_header_buffer[0] = 0x28;
    cmd_and_header_buffer[1] = 0xA2;

    cmd_and_header_buffer[2..].copy_from_slice(&tx_message.header.into_bytes());

    for chunk in cmd_and_header_buffer[2..].chunks_exact_mut(4) {
        let num = BigEndian::read_u32(chunk);
        LittleEndian::write_u32(chunk, num);
    }

    mocks.expect_fifo_write_transaction(cmd_and_header_buffer, payload, &mut seq);

    mocks.expect_register_write([0x20, 0x69, 0x03], &mut seq);

    // mock reading of fifo control register
    // 1st attempt -> txreq still set ->not all messages inside tx fifo have been transmitted
    mocks.mock_register_read::<0x02>([0x30, 0x69], &mut seq);
    // 2nd attempt -> txreq cleared -> all messages inside tx fifo have been transmitted
    mocks.mock_register_read::<0x00>([0x30, 0x69], &mut seq);

    mocks.into_controller().transmit(&tx_message_copy, true).unwrap();
}

#[test]
fn test_read_fifo_invalid_payload_buffer_size() {
    let mocks = Mocks::default();
    let mut buff = [0u8; 3];

    let result = mocks.into_controller().read_fifo(0x123, &mut buff);
    assert_eq!(result.unwrap_err(), CanError::InvalidBufferSize(3));
}

#[test]
fn test_receive() {
    let mut mocks = Mocks::default();

    let mut seq = Sequence::new();

    let mut message_buff = [0u8; 8];

    // status register read (wait till fifo not empty flag is set)
    mocks.mock_register_read::<0b0000_0000>([0x30, 0x60], &mut seq);

    // status register read (fifo not empty flag is set)
    mocks.mock_register_read::<0b0000_0001>([0x30, 0x60], &mut seq);

    // user address register read
    mocks.mock_read32::<0x00_00_04_7C>([0x30, 0x64], &mut seq);

    // Message read from RAM address (0x47C+8) to start reading received message object payload
    // transfer cmd+address
    mocks.expect_fifo_read_transaction([0x38, 0x84], [1, 2, 3, 4, 5, 6, 7, 8], &mut seq);

    mocks.expect_register_write([0x20, 0x5D, 0b0000_0001], &mut seq);

    let result = mocks.into_controller().receive(&mut message_buff, true);

    assert!(result.is_ok());

    assert_eq!(message_buff, [1, 2, 3, 4, 5, 6, 7, 8]);
}

#[test]
fn test_receive_fifo_empty() {
    let mut mocks = Mocks::default();

    let mut seq = Sequence::new();

    let mut message_buff = [0u8; 8];

    // status register read (fifo not empty flag is not set)
    mocks.mock_register_read::<0b0000_0000>([0x30, 0x60], &mut seq);

    let result = mocks.into_controller().receive(&mut message_buff, false);

    assert_eq!(result.unwrap_err(), CanError::RxFifoEmptyErr);
}

#[test]
fn test_transmit_fifo_full() {
    let mut mocks = Mocks::default();
    let mut seq = Sequence::new();
    let payload: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    let payload_bytes = Bytes::copy_from_slice(&payload);

    let msg_type = Can20::<8> {};

    let identifier = ExtendedId::new(EXTENDED_ID).unwrap();
    let tx_message = TxMessage::new(msg_type, payload_bytes, Id::Extended(identifier)).unwrap();

    // mock fifo status register read byte 0 (1st attempt) -> tx fifo full
    mocks.mock_register_read::<0b0000_0000>([0x30, 0x6C], &mut seq);

    let res = mocks.into_controller().transmit(&tx_message, false);

    assert_eq!(res.unwrap_err(), CanError::TxFifoFullErr);
}

#[test]
fn test_reset_command() {
    let mut mocks = Mocks::default();
    let mut seq = Sequence::new();
    mocks.expect_register_write([0x0; 3], &mut seq);

    mocks.into_controller().reset().unwrap();
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

    let mut mock = Mocks::new();
    let mut seq = Sequence::new();

    // Request configuration mode
    mock.expect_register_write([0x20, 0x3, 0b0000_1100], &mut seq);

    // Still in normal mode
    mock.mock_register_read::<0b0001_0100>([0x30, 0x2], &mut seq);

    // Configuration mode
    mock.mock_register_read::<0b1001_0100>([0x30, 0x2], &mut seq);

    expect_config(&mut mock, &mut seq);

    // Request normal CAN FD mode
    mock.expect_register_write([0x20, 0x3, 0b0000_1000], &mut seq);

    // Still configuration mode
    mock.mock_register_read::<0b1001_0100>([0x30, 0x2], &mut seq);
    // Still configuration mode
    mock.mock_register_read::<0b1001_0100>([0x30, 0x2], &mut seq);

    match mock
        .into_controller()
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
                mode: RequestMode::NormalCANFD,
                bit_rate: BitRateConfig::default(),
            },
            &clock,
        )
        .unwrap_err()
    {
        CanError::RequestModeTimeout => {}
        _ => panic!("unexpected error type"),
    }
}

#[test]
fn test_configure_transfer_error() {
    let clock = TestClock::new(vec![]);
    let mut mock = Mocks::default();
    mock.mock_transfer_error();

    match mock.into_controller().configure(&Configuration::default(), &clock).unwrap_err() {
        CanError::BusErr(_) => {}
        _ => panic!("unexpected error type"),
    }
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
fn test_read_operation_status_transfer_error() {
    let mut mocks = Mocks::default();
    mocks.mock_transfer_error();

    match mocks.into_controller().read_operation_status().unwrap_err() {
        CanError::BusErr(_) => {}
        _ => panic!("Unexpected error type"),
    }
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
fn test_read_oscillator_transfer_error() {
    let mut mocks = Mocks::default();
    mocks.mock_transfer_error();

    match mocks.into_controller().read_oscillator_status().unwrap_err() {
        CanError::BusErr(_) => {}
        _ => panic!("Unexpected error type"),
    }
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
fn test_read_clock_configuration_transfer_error() {
    let mut mocks = Mocks::default();
    mocks.mock_transfer_error();

    match mocks.into_controller().read_clock_configuration().unwrap_err() {
        CanError::BusErr(_) => {}
        _ => panic!("Unexpected error type"),
    }
}

#[test]
fn test_filter_enable() {
    let mut mocks = Mocks::default();
    let mut seq = Sequence::new();

    mocks.expect_register_write([0x21, 0xD2, 0x00], &mut seq);

    // write the fifo index where the message that matches the filter is stored
    // Fifo rx index is 1 in our case

    mocks.expect_register_write([0x21, 0xD2, 0x01], &mut seq);

    mocks.expect_register_write([0x21, 0xD2, 0x81], &mut seq);

    let result = mocks.into_controller().enable_filter(1, 2);

    assert!(result.is_ok());
}

#[test]
fn test_filter_disable() {
    let mut mocks = Mocks::default();
    let mut seq = Sequence::new();

    // Disable filter 6
    mocks.expect_register_write([0x21, 0xD6, 0x00], &mut seq);

    let result = mocks.into_controller().disable_filter(6);

    assert!(result.is_ok());
}

#[derive(Default, Debug, PartialEq)]
pub(crate) struct Mocks {
    pub(crate) device: MockSPIDevice,
}

impl Mocks {
    pub fn new() -> Self {
        Self {
            device: MockSPIDevice::new(),
        }
    }
    pub fn into_controller(self) -> MCP2517<MockSPIDevice, TestClock> {
        MCP2517::new(self.device)
    }

    /// Simulates a SPI transfer fault
    pub fn mock_transfer_error(&mut self) {
        self.device.expect_transaction().times(1).return_const(Err(SPIError::Error1));
    }

    /// Mocks the reading of a single register byte
    pub fn mock_register_read<const REG: u8>(&mut self, expected_command: [u8; 2], seq: &mut Sequence) {
        let expected_buffer = [expected_command[0], expected_command[1], 0x0];

        self.device
            .expect_transaction()
            .times(1)
            .returning(move |operation| {
                assert_eq!(operation.len(), 1);
                match &mut operation[0] {
                    Operation::TransferInPlace(buff) => {
                        assert_eq!(expected_buffer, *buff);
                        buff.copy_from_slice(&[0x0, 0x0, REG]);
                    }
                    _ => panic!("unexpected operation {:?}", operation[0]),
                }
                Ok(())
            })
            .in_sequence(seq);
    }

    /// mocks 4-byte register read
    pub fn mock_read32<const REG: u32>(&mut self, expected_command: [u8; 2], seq: &mut Sequence) {
        let expected_buffer = [expected_command[0], expected_command[1]];

        self.device
            .expect_transaction()
            .times(1)
            .returning(move |operation| {
                assert_eq!(operation.len(), 2);
                match &operation[0] {
                    Operation::Write(buff) => {
                        assert_eq!(expected_buffer, *buff);
                    }
                    _ => panic!("unexpected operation {:?}", operation[0]),
                }
                match &mut operation[1] {
                    Operation::Read(read) => {
                        assert_eq!(read.len(), 4);
                        read.copy_from_slice(&[REG as u8, (REG >> 8) as u8, (REG >> 16) as u8, (REG >> 24) as u8]);
                    }
                    _ => panic!("unexpected operation {:?}", operation[1]),
                }
                Ok(())
            })
            .in_sequence(seq);
    }

    /// Mock write of single register (1 byte) using SPI transfer
    pub fn expect_register_write(&mut self, expected_write: [u8; 3], sequence: &mut Sequence) {
        self.device
            .expect_transaction()
            .times(1)
            .returning(move |operation| {
                assert_eq!(operation.len(), 1);

                match &operation[0] {
                    Operation::TransferInPlace(buff) => {
                        assert_eq!(expected_write, *buff);
                    }
                    _ => panic!("Unexpected operation received {:?}", operation[0]),
                }

                Ok(())
            })
            .in_sequence(sequence);
    }

    /// Mock write 4-byte register write
    pub fn mock_write32(&mut self, expected_write: [u8; 6], sequence: &mut Sequence) {
        self.device
            .expect_transaction()
            .times(1)
            .returning(move |operation| {
                assert_eq!(operation.len(), 1);

                match operation[0] {
                    Operation::Write(write) => {
                        assert_eq!(write, expected_write);
                    }
                    _ => panic!("Unexpected operation received {:?}", operation[0]),
                }

                Ok(())
            })
            .in_sequence(sequence);
    }

    /// Mock write operation to TX FIFO
    pub fn expect_fifo_write_transaction<const L: usize>(
        &mut self,
        header: [u8; 10],
        payload: [u8; L],
        seq: &mut Sequence,
    ) {
        self.device
            .expect_transaction()
            .times(1)
            .returning(move |operation| {
                assert_eq!(operation.len(), 2);
                match &operation[0] {
                    Operation::Write(write) => {
                        assert_eq!(*write, header);
                    }
                    _ => panic!("Unexpected operation received {:?}", operation[0]),
                }
                match operation[1] {
                    Operation::Write(write) => {
                        assert_eq!(write[..payload.len()], payload);
                    }
                    _ => panic!("Unexpected operation received {:?}", operation[0]),
                }
                Ok(())
            })
            .in_sequence(seq);
    }

    /// Mock read operation of RX FIFO
    pub fn expect_fifo_read_transaction<const L: usize>(
        &mut self,
        command: [u8; 2],
        payload_received: [u8; L],
        seq: &mut Sequence,
    ) {
        self.device
            .expect_transaction()
            .times(1)
            .returning(move |operation| {
                assert_eq!(operation.len(), 2);
                match operation[0] {
                    Operation::Write(write) => {
                        assert_eq!(write, command);
                    }
                    _ => panic!("Unexpected operation received {:?}", operation[0]),
                }

                match &mut operation[1] {
                    Operation::Read(read) => {
                        read.copy_from_slice(&payload_received);
                    }
                    _ => panic!("Unexpected operation received {:?}", operation[0]),
                }
                Ok(())
            })
            .in_sequence(seq);
    }
}

#[test]
fn test_lib() {
    let spi_dev = ExampleSPIDevice::default();
    let clock = ExampleClock::default();

    let mut controller = MCP2517::new(spi_dev);

    // configure CAN controller
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
                mode: RequestMode::NormalCANFD,
                bit_rate: BitRateConfig {
                    sys_clk: SysClk::MHz20,
                    can_speed: CanBaudRate::Kpbs500,
                },
            },
            &clock,
        )
        .unwrap();

    // Create message frame
    let can_id = Id::Standard(StandardId::new(0x55).unwrap());

    // Important note: Generic arg for message type for CAN2.0
    // should be either 4 or 8, the DLC will be based off the
    // length of the payload buffer. So for a payload of 5 bytes
    // you can only use Can20::<8> as the message type
    let message_type = Can20::<8> {};
    let payload = [0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8];
    let pl_bytes = Bytes::copy_from_slice(&payload);
    let can_message = TxMessage::new(message_type, pl_bytes, can_id).unwrap();

    // Create and set filter object
    let filter = Filter::new(can_id, 0).unwrap();
    let _ = controller.set_filter_object(filter);

    // Transmit CAN message in blocking mode
    controller.transmit(&can_message, true).unwrap();

    // Receive CAN message in blocking mode
    let mut buff = [0u8; 8];
    let result = controller.receive(&mut buff, true);
    assert!(result.is_ok());
    assert_eq!(buff, [0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8]);
}
