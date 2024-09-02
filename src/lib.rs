#![cfg_attr(not(test), no_std)]
#![cfg_attr(feature = "strict", deny(warnings))]
#![allow(dead_code)]
#![allow(clippy::identity_op)]

extern crate alloc;

pub mod can;
pub mod config;
pub mod status;

pub mod filter;
pub mod message;

pub mod example;
#[cfg(test)]
pub(crate) mod mocks;
mod registers;
#[cfg(test)]
mod tests;
