//!# CAN Filter
//! The [Filter] object is used to create a CAN filter. The MCP2517FD CAN chip has 32 filter/mask registers.
//! Lower index of the filter means higher priority (highest priority =0, lowest priority = 31).
//!
//! ```
//!# use mcp2517::filter::Filter;
//!# use embedded_can::{Id,ExtendedId};
//!
//! // ID to match
//! let id = Id::Extended(ExtendedId::new(0xC672).unwrap());
//! // Create filter with index 2
//! let mut filter = Filter::new(id,2).unwrap();
//! // Set mask MSB bits, so that only the MSB of the message ID needs to match the filter
//! filter.set_mask_extended_id(0xFF00);
//!
//!
use crate::message::{EXTENDED_IDENTIFIER_MASK, STANDARD_IDENTIFIER_MASK};
use crate::registers::{FilterMaskReg, FilterObjectReg};
use embedded_can::{ExtendedId, Id, StandardId};

/// Struct representing a filter object
#[derive(Default, Debug)]
pub struct Filter {
    /// filter & mask index
    pub(crate) index: u8,
    /// mask register bitfield
    pub(crate) mask_bits: FilterMaskReg,
    /// filter register bitfield
    pub(crate) filter_bits: FilterObjectReg,
}

impl Filter {
    /// Create new filter from [embedded_can::Id] and index, no mask
    pub fn new(identifier: Id, index: u8) -> Option<Self> {
        if index > 31 {
            return None;
        }

        let mut filter = Self::default();

        filter.set_id(identifier);
        filter.index = index;

        Some(filter)
    }

    /// Set mask for extended Id
    pub fn set_mask_extended_id(&mut self, mask: u32) {
        self.set_mask(Id::Extended(ExtendedId::new(mask).unwrap()));
    }

    /// Set mask for standard Id
    pub fn set_mask_standard_id(&mut self, mask: u16) {
        self.set_mask(Id::Standard(StandardId::new(mask).unwrap()));
    }

    /// Set filter and mask so that only messages with Standard Id match
    pub fn match_standard_only(&mut self) {
        self.mask_bits.set_mide(true);
        self.filter_bits.set_exide(false);
    }

    /// Set filter and mask so that only messages with Extended Id match
    pub fn match_extended_only(&mut self) {
        self.mask_bits.set_mide(true);
        self.filter_bits.set_exide(true);
    }

    fn set_id(&mut self, identifier: Id) {
        match identifier {
            Id::Standard(sid) => self.filter_bits.set_sid(sid.as_raw()),
            Id::Extended(eid) => {
                self.filter_bits.set_eid(eid.as_raw() & EXTENDED_IDENTIFIER_MASK);
                self.filter_bits.set_sid((eid.as_raw() >> 18) as u16 & STANDARD_IDENTIFIER_MASK);
            }
        }
    }

    fn set_mask(&mut self, identifier: Id) {
        match identifier {
            Id::Standard(sid) => self.mask_bits.set_msid(sid.as_raw()),
            Id::Extended(eid) => {
                self.mask_bits.set_meid(eid.as_raw() & EXTENDED_IDENTIFIER_MASK);
                self.mask_bits.set_msid((eid.as_raw() >> 18) as u16 & STANDARD_IDENTIFIER_MASK);
            }
        }
    }
}
