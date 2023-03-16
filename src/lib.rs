#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod can;
pub mod config;
pub mod status;

#[cfg(test)]
pub(crate) mod mocks;
#[cfg(test)]
mod tests;
