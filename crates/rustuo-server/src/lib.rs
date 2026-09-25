#![forbid(unsafe_code)]

pub mod account_repository;
pub mod reconnect_admission;

/// Application port for account credential verification.
///
/// Implementations receive borrowed raw bytes and are independent of storage
/// and transport concerns.
pub trait ReconnectCredentialVerifier {
    type Account;
    type Error;

    fn verify(&mut self, username: &[u8], password: &[u8]) -> Result<Self::Account, Self::Error>;
}
