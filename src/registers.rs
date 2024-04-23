use modular_bitfield_msb::prelude::*;

/// Fourth byte of FIFO Control register
#[bitfield]
#[derive(Default)]
#[repr(u8)]
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

/// Third byte of FIFO Control register
#[bitfield]
#[derive(Default)]
#[repr(u8)]
pub struct FifoControlReg2 {
    #[skip]
    __: B1,
    pub txat: B2,
    pub txpri: B5,
}

/// Second byte of FIFO Control register
#[bitfield]
#[derive(Default)]
#[repr(u8)]
pub struct FifoControlReg1 {
    #[skip]
    __: B5,
    pub freset: bool,
    pub txreq: bool,
    pub uinc: bool,
}

/// First byte of FIFO Control register
#[bitfield]
#[derive(Default)]
#[repr(u8)]
pub struct FifoControlReg0 {
    pub txen: bool,
    pub rtren: bool,
    pub rxtsen: bool,
    pub txatie: bool,
    pub rxovie: bool,
    pub tferffie: bool,
    pub tfhrfhie: bool,
    pub tfnrfnie: bool,
}

/// Second byte of FIFO Status register
#[bitfield]
#[derive(Default)]
#[repr(u8)]
pub struct FifoStatusReg1 {
    #[skip]
    __: B3,
    pub fifoci: B5,
}
/// First byte of FIFO Status register

#[bitfield]
#[derive(Default)]
#[repr(u8)]
pub struct FifoStatusReg0 {
    pub txabt: bool,
    pub txlarb: bool,
    pub txerr: bool,
    pub txatif: bool,
    pub rxovif: bool,
    pub tferffif: bool,
    pub tfhrfhif: bool,
    pub tfnrfnif: bool,
}
/// filter mask
#[bitfield]
#[derive(Default, Debug, Eq, PartialEq)]
#[repr(u32)]
pub struct FilterMaskReg {
    #[skip]
    __: B1,
    pub mide: bool,
    pub msid11: bool,
    pub meid: B18,
    pub msid: B11,
}

/// filter object
#[bitfield]
#[derive(Default, Debug, Eq, PartialEq)]
#[repr(u32)]
pub struct FilterObjectReg {
    #[skip]
    __: B1,
    pub exide: bool,
    pub sid11: bool,
    pub eid: B18,
    pub sid: B11,
}
