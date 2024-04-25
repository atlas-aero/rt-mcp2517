#![cfg_attr(not(test), no_std)]
#![allow(dead_code)]

extern crate alloc;

pub mod can;
pub mod config;
pub mod status;

pub mod filter;
pub mod message;
#[cfg(test)]
pub(crate) mod mocks;
mod registers;
#[cfg(test)]
mod tests;
