use modular_bitfield_msb::prelude::*;
/// Fourth byte of FIFO Control register
#[bitfield]
pub struct FifoControlReg3 {
    pub plsize: B3,
    fsize: B5,
}
impl FifoControlReg3 {
    /// set FIFO size (number of messages 1-32)
    pub fn with_fifo_size(mut self, value: u8) -> Self {
        let size = value.max(1).min(32);
        self.set_fsize(size - 1);
        self
    }
    /// get FIFO size
    pub fn get_fifo_size(&self) -> u8 {
        self.fsize() + 1
    }
}

/// Third byte of FIFO Control register
#[bitfield]
pub struct FifoControlReg2 {
    #[skip]
    __: B1,
    pub txat: B2,
    pub txpri: B5,
}

/// Second byte of FIFO Control register
#[bitfield]
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
#[allow(dead_code)]
pub struct FifoStatusReg1 {
    #[skip]
    __: B3,
    pub fifoci: B5,
}
/// First byte of FIFO Status register

#[allow(dead_code)]
#[bitfield]
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
