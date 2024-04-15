#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod can;
pub mod config;
pub mod status;

mod message;
#[cfg(test)]
pub(crate) mod mocks;
mod registers;
#[cfg(test)]
mod tests;
