use crate::config::{PayloadSize, RetransmissionAttempts};
use crate::registers::*;
#[test]
fn test_fifocontrolreg0() {
    assert_eq!([0b1000_0000], FifoControlReg0::new().with_txen(true).into_bytes());
}

#[test]
fn test_fifocontrolreg1() {
    assert_eq!(
        [0b0000_0011],
        FifoControlReg1::new().with_uinc(true).with_txreq(true).into_bytes()
    );
}

#[test]
fn test_fifocontrolreg2() {
    assert_eq!(
        [0b0100_0011],
        FifoControlReg2::new()
            .with_txat(RetransmissionAttempts::Unlimited as u8)
            .with_txpri(3)
            .into_bytes()
    );
}

#[test]
fn test_fifocontrolreg3() {
    let fifo_control_reg3 = FifoControlReg3::new()
        .with_plsize(PayloadSize::TwentyFourBytes as u8)
        .with_fifo_size(32);

    assert_eq!(32, fifo_control_reg3.get_fifo_size());
    assert_eq!([0b1001_1111], fifo_control_reg3.into_bytes());
}

#[test]
fn test_fifostatusreg0() {
    assert_eq!([0b0000_0001], FifoStatusReg0::new().with_tfnrfnif(true).into_bytes());
}
