#![forbid(unsafe_code)]

//! Shared RustUO server primitives.

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Serial(pub u32);
