use crate::can::CanController;
use crate::filter::Filter;
use crate::tests::can::Mocks;
use embedded_can::{ExtendedId, Id, StandardId};
use mockall::Sequence;

const EXTENDED_ID: u32 = 0x14C92A2B; //0b000(1_0100_1100_10)(01_0010_1010_0010_1011)
const STANDARD_ID: u16 = 0x6A5;

#[test]
fn test_set_filter_object_standard_id() {
    let id_standard = StandardId::new(STANDARD_ID).unwrap();
    let mut filter = Filter::new(Id::Standard(id_standard), 1).unwrap();

    let mut seq = Sequence::new();

    // mask 2 lsb of standard id -> MSID <1:0> should be set
    filter.set_mask_standard_id(0b000_0000_0011);

    // MIDE should be set and EXIDE should be cleared
    filter.match_standard_only();

    let mut mocks = Mocks::default();

    // disable filter 0
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
            assert_eq!([0x21, 0xD1, 0x00], data);
            Ok(&[0u8; 3])
        })
        .in_sequence(&mut seq);
    mocks
        .pin_cs
        .expect_set_high()
        .times(1)
        .return_const(Ok(()))
        .in_sequence(&mut seq);

    // write filter value
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
            assert_eq!([0x21, 0xF8, 0xA5, 0x6, 0x0, 0x0], data);
            Ok(&[0u8; 2])
        })
        .in_sequence(&mut seq);
    mocks
        .pin_cs
        .expect_set_high()
        .times(1)
        .return_const(Ok(()))
        .in_sequence(&mut seq);

    // write mask value
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
            assert_eq!([0x21, 0xFC, 0x3, 0u8, 0u8, 0x40], data);
            Ok(&[0u8; 6])
        })
        .in_sequence(&mut seq);
    mocks
        .pin_cs
        .expect_set_high()
        .times(1)
        .return_const(Ok(()))
        .in_sequence(&mut seq);

    // enable filter
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
            assert_eq!([0x21, 0xD1, 0x81], data);
            Ok(&[0u8; 6])
        })
        .in_sequence(&mut seq);
    mocks
        .pin_cs
        .expect_set_high()
        .times(1)
        .return_const(Ok(()))
        .in_sequence(&mut seq);

    let result = mocks.into_controller().set_filter_object(filter);

    assert!(result.is_ok());
}

#[test]
fn test_set_filter_object_extended_id() {
    let id_extended = ExtendedId::new(EXTENDED_ID).unwrap();
    let mut filter = Filter::new(Id::Extended(id_extended), 0).unwrap();

    let mut seq = Sequence::new();

    // mask the 2 msb of extended id -> MSID<10:9> should be set
    filter.set_mask_extended_id(0b1_1000_0000_0000_0000_0000_0000_0000);

    let mut mocks = Mocks::default();

    // disable filter 0
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
            assert_eq!([0x21, 0xD0, 0x00], data);
            Ok(&[0u8; 3])
        })
        .in_sequence(&mut seq);
    mocks
        .pin_cs
        .expect_set_high()
        .times(1)
        .return_const(Ok(()))
        .in_sequence(&mut seq);

    // write filter value
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
            assert_eq!([0x21, 0xF0, 0x32, 0x5D, 0x51, 0x09], data);
            Ok(&[0u8; 2])
        })
        .in_sequence(&mut seq);
    mocks
        .pin_cs
        .expect_set_high()
        .times(1)
        .return_const(Ok(()))
        .in_sequence(&mut seq);

    // write mask value
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
            assert_eq!([0x21, 0xF4, 0u8, 0x6, 0u8, 0u8], data);
            Ok(&[0u8; 6])
        })
        .in_sequence(&mut seq);
    mocks
        .pin_cs
        .expect_set_high()
        .times(1)
        .return_const(Ok(()))
        .in_sequence(&mut seq);

    // enable filter
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
            assert_eq!([0x21, 0xD0, 0x81], data);
            Ok(&[0u8; 6])
        })
        .in_sequence(&mut seq);
    mocks
        .pin_cs
        .expect_set_high()
        .times(1)
        .return_const(Ok(()))
        .in_sequence(&mut seq);

    let result_extended = mocks.into_controller().set_filter_object(filter);

    assert!(result_extended.is_ok());
}
