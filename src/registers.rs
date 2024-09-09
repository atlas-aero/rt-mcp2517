#![allow(unused_braces)]
use modular_bitfield_msb::prelude::*;

#[bitfield]
#[derive(Default)]
#[repr(u8)]
/// Fourth byte of FIFO Control register
pub struct FifoControlReg3 {
    pub plsize: B3,
    fsize: B5,
}

impl FifoControlReg3 {
    /// set FIFO size (number of messages 1-32)
    pub fn with_fifo_size(mut self, value: u8) -> Self {
        let size = value.clamp(1, 32);
        self.set_fsize(size - 1);
        self
    }

    /// get FIFO size
    pub fn fifo_size(&self) -> u8 {
        self.fsize() + 1
    }
}

#[bitfield]
#[derive(Default)]
#[repr(u8)]
/// Third byte of FIFO Control register
pub struct FifoControlReg2 {
    #[skip]
    __: B1,
    /// Retransmission attempts bits
    pub txat: B2,
    /// Message transmit priority bits
    pub txpri: B5,
}

#[bitfield]
#[derive(Default)]
#[repr(u8)]
/// Second byte of FIFO Control register
pub struct FifoControlReg1 {
    #[skip]
    __: B5,
    /// FIFO Reset bit
    pub freset: bool,
    /// Message Send Request bit
    pub txreq: bool,
    /// Increment FIFO Head/Tail bit
    pub uinc: bool,
}

#[bitfield]
#[derive(Default)]
#[repr(u8)]
/// First byte of FIFO Control register
pub struct FifoControlReg0 {
    /// TX/RX FIFO Selection bit
    pub txen: bool,
    /// Auto RTR Enable bit
    pub rtren: bool,
    /// Received Message Time Stamp Enable bit
    pub rxtsen: bool,
    /// Transmit Attempts Exhausted Interrupt Enable bit
    pub txatie: bool,
    /// Overflow Interrupt Enable bit
    pub rxovie: bool,
    /// Transmit/Receive FIFO Empty/Full Interrupt Enable bit
    pub tferffie: bool,
    /// Transmit/Receive FIFO Half Empty/Half Full Interrupt Enable bit
    pub tfhrfhie: bool,
    /// Transmit/Receive FIFO Not Full/Not ETransmit/Receive FIFO Not Full/Not Empty Interrupt Flag bitmpty Interrupt Enable bit
    pub tfnrfnie: bool,
}

#[bitfield]
#[derive(Default)]
#[repr(u8)]
/// Second byte of FIFO Status register
pub struct FifoStatusReg1 {
    #[skip]
    __: B3,
    /// FIFO Message Index bits
    pub fifoci: B5,
}

#[bitfield]
#[derive(Default)]
#[repr(u8)]
/// First byte of FIFO Status register
pub struct FifoStatusReg0 {
    /// Message Aborted Status bit
    pub txabt: bool,
    /// Message Lost Arbitration Status bit
    pub txlarb: bool,
    /// Error Detected During Transmission bit
    pub txerr: bool,
    /// Transmit Attempts Exhausted Interrupt Pending bit
    pub txatif: bool,
    /// Receive FIFO Overflow Interrupt Flag bit
    pub rxovif: bool,
    /// Transmit/Receive FIFO Empty/Full Interrupt Flag bit
    pub tferffif: bool,
    /// Transmit/Receive FIFO Half Empty/Half Full Interrupt Flag bit
    pub tfhrfhif: bool,
    /// Transmit/Receive FIFO Not Full/Not Empty Interrupt Flag bit
    pub tfnrfnif: bool,
}

/// Filter mask register
#[bitfield]
#[derive(Default, Debug, Eq, PartialEq)]
#[repr(u32)]
pub struct FilterMaskReg {
    #[skip]
    __: B1,
    /// Identifier Receive mode bit,
    /// if 1, match either standard or extended (corresponding to EXIDE bit in filter),
    /// if 0 match either
    pub mide: bool,
    /// Standard ID Mask bit
    pub msid11: bool,
    /// Extended ID Mask bits
    pub meid: B18,
    /// Standard ID Mask bits
    pub msid: B11,
}

/// Filter object register
#[bitfield]
#[derive(Default, Debug, Eq, PartialEq)]
#[repr(u32)]
pub struct FilterObjectReg {
    #[skip]
    __: B1,
    /// Extended ID enable bit
    /// If MIDE 1, setting this bit matches Extended ID only
    /// If MIDE 0, clearing this bit matches Standard ID only
    pub exide: bool,
    /// Standard ID filter bit
    pub sid11: bool,
    /// Extended ID filter bits
    pub eid: B18,
    /// Standard ID filter rbits
    pub sid: B11,
}

/// Nominal bit time configuration register
#[bitfield]
#[derive(Default, Debug, Eq, PartialEq)]
#[repr(u32)]
pub struct C1NBTCFG {
    /// Baud rate prescalar bits
    pub brp: B8,
    /// Time Segment 1 bits (Propagation Segment + Phase Segment 1)
    pub tseg1: B8,
    #[skip]
    __: B1,
    /// Time Segment 2 bits (Phase Segment 2)
    pub tseg2: B7,
    #[skip]
    __: B1,
    /// Synchronization Jump Width bits
    pub sjw: B7,
}

impl C1NBTCFG {
    pub fn to_bytes(&mut self, values: [u8; 4]) {
        self.set_brp(values[0]);
        self.set_tseg1(values[1]);
        self.set_tseg2(values[2]);
        self.set_sjw(values[3]);
    }
}
