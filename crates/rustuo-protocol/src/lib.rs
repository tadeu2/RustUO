#![forbid(unsafe_code)]

//! Ultima Online protocol boundary.

use rustuo_core::Serial;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityReference {
    pub serial: Serial,
}

/// A decoded game-login packet carrying the reconnect grant and credentials.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenaissanceGameLogin<'a> {
    pub auth_id: u32,
    pub username: &'a [u8],
    pub password: &'a [u8],
}
