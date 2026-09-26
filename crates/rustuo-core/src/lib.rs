#![forbid(unsafe_code)]

//! Shared RustUO server primitives.

use std::fmt;
use std::str::FromStr;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Serial(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CoreError {
    MalformedClientVersion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClientVersion {
    pub major: u32,
    pub minor: u32,
    pub revision: u32,
    pub build: u32,
}

impl ClientVersion {
    pub const fn new(major: u32, minor: u32, revision: u32, build: u32) -> Self {
        Self {
            major,
            minor,
            revision,
            build,
        }
    }
}

impl FromStr for ClientVersion {
    type Err = CoreError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let mut values = [0; 4];
        let mut count = 0;
        for component in text.split('.') {
            if count == values.len() {
                return Err(CoreError::MalformedClientVersion);
            }
            if component.is_empty() || !component.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(CoreError::MalformedClientVersion);
            }
            values[count] = component
                .parse()
                .map_err(|_| CoreError::MalformedClientVersion)?;
            count += 1;
        }
        if count != values.len() {
            return Err(CoreError::MalformedClientVersion);
        }
        Ok(Self::new(values[0], values[1], values[2], values[3]))
    }
}

impl fmt::Display for ClientVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}.{}.{}.{}",
            self.major, self.minor, self.revision, self.build
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{ClientVersion, CoreError};
    use std::str::FromStr;

    #[test]
    fn client_versions_parse_compare_and_format_canonically() {
        let version = ClientVersion::from_str("5.0.8.3").unwrap();
        assert_eq!(version, ClientVersion::new(5, 0, 8, 3));
        assert_eq!(version.to_string(), "5.0.8.3");
        assert_eq!(ClientVersion::from_str("5.0.8.3"), Ok(version));
        assert!(ClientVersion::new(5, 0, 8, 3) < ClientVersion::new(5, 0, 8, 4));
        assert!(ClientVersion::new(5, 1, 0, 0) > ClientVersion::new(5, 0, 99, 99));

        for text in [
            "",
            "5.0.8",
            "5.0.8.3.1",
            "5..8.3",
            "5.a.8.3",
            "-1.0.0.0",
            "4294967296.0.0.0",
        ] {
            assert_eq!(
                ClientVersion::from_str(text),
                Err(CoreError::MalformedClientVersion)
            );
        }
    }
}
