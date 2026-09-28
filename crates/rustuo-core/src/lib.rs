#![forbid(unsafe_code)]

//! Shared RustUO server primitives.

use std::fmt;
use std::str::FromStr;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Serial(pub u32);

impl Serial {
    pub const fn is_sentinel(self) -> bool {
        self.0 == 0 || self.0 == u32::MAX
    }

    pub const fn is_mobile(self) -> bool {
        self.0 >= 1 && self.0 < 0x4000_0000
    }

    pub const fn is_item(self) -> bool {
        self.0 >= 0x4000_0000 && self.0 <= 0x7fff_ffff
    }

    pub const fn try_allocatable(self) -> Result<Self, CoreError> {
        if self.is_sentinel() {
            Err(CoreError::SentinelSerial { raw: self.0 })
        } else if self.is_mobile() || self.is_item() {
            Ok(self)
        } else {
            Err(CoreError::InvalidSerial { raw: self.0 })
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CoreError {
    SentinelSerial { raw: u32 },
    InvalidSerial { raw: u32 },
    InvalidEntityId { raw: u32 },
    UnknownDirectionBits { raw: u8 },
    MalformedClientVersion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId(Serial);

impl EntityId {
    pub const fn new(raw: u32) -> Result<Self, CoreError> {
        match Serial(raw).try_allocatable() {
            Ok(serial) => Ok(Self(serial)),
            Err(CoreError::InvalidSerial { .. }) => Err(CoreError::InvalidEntityId { raw }),
            Err(error) => Err(error),
        }
    }

    pub const fn raw(self) -> u32 {
        self.serial().0
    }

    pub const fn serial(self) -> Serial {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Point3 {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Point3 {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MapId(u8);

impl MapId {
    pub const fn new(raw: u8) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u8 {
        self.0
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Direction {
    North = 0,
    Right = 1,
    East = 2,
    Down = 3,
    South = 4,
    Left = 5,
    West = 6,
    Up = 7,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DecodedDirection {
    base: Direction,
    running: bool,
}

impl Direction {
    pub const fn from_raw(raw: u8) -> Result<DecodedDirection, CoreError> {
        if raw & !0x87 != 0 {
            return Err(CoreError::UnknownDirectionBits { raw });
        }

        let base = match raw & 0x07 {
            0 => Self::North,
            1 => Self::Right,
            2 => Self::East,
            3 => Self::Down,
            4 => Self::South,
            5 => Self::Left,
            6 => Self::West,
            _ => Self::Up,
        };
        Ok(DecodedDirection {
            base,
            running: raw & 0x80 != 0,
        })
    }

    pub const fn offset(self) -> (i32, i32) {
        match self {
            Self::North => (0, -1),
            Self::Right => (1, -1),
            Self::East => (1, 0),
            Self::Down => (1, 1),
            Self::South => (0, 1),
            Self::Left => (-1, 1),
            Self::West => (-1, 0),
            Self::Up => (-1, -1),
        }
    }
}

impl DecodedDirection {
    pub const fn base(self) -> Direction {
        self.base
    }

    pub const fn is_running(self) -> bool {
        self.running
    }

    pub const fn to_raw(self) -> u8 {
        self.base as u8 | if self.running { 0x80 } else { 0 }
    }

    pub const fn offset(self) -> (i32, i32) {
        self.base.offset()
    }
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
    use super::{ClientVersion, CoreError, Direction, EntityId, MapId, Point3, Serial};
    use std::collections::HashSet;
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

    #[test]
    fn entity_id_accepts_only_allocatable_serials_and_preserves_raw_value() {
        for raw in [1, 0x3fff_ffff, 0x4000_0000, 0x7fff_ffff] {
            assert_eq!(EntityId::new(raw).unwrap().raw(), raw);
        }
        assert_eq!(EntityId::new(0), Err(CoreError::SentinelSerial { raw: 0 }));
        assert_eq!(
            EntityId::new(u32::MAX),
            Err(CoreError::SentinelSerial { raw: u32::MAX })
        );
        assert_eq!(
            EntityId::new(0x8000_0000),
            Err(CoreError::InvalidEntityId { raw: 0x8000_0000 })
        );
    }

    #[test]
    fn serial_classification_covers_sentinels_mobile_item_and_reserved_boundaries() {
        for raw in [0, u32::MAX] {
            let serial = Serial(raw);
            assert!(serial.is_sentinel());
            assert!(!serial.is_mobile());
            assert!(!serial.is_item());
            assert_eq!(
                serial.try_allocatable(),
                Err(CoreError::SentinelSerial { raw })
            );
        }
        for raw in [1, 0x3fff_ffff] {
            let serial = Serial(raw);
            assert!(!serial.is_sentinel());
            assert!(serial.is_mobile());
            assert!(!serial.is_item());
            assert_eq!(serial.try_allocatable(), Ok(serial));
        }
        for raw in [0x4000_0000, 0x7fff_ffff] {
            let serial = Serial(raw);
            assert!(!serial.is_sentinel());
            assert!(!serial.is_mobile());
            assert!(serial.is_item());
            assert_eq!(serial.try_allocatable(), Ok(serial));
        }
        for raw in [0x8000_0000, 0xffff_fffe] {
            let serial = Serial(raw);
            assert!(!serial.is_sentinel());
            assert!(!serial.is_mobile());
            assert!(!serial.is_item());
            assert_eq!(
                serial.try_allocatable(),
                Err(CoreError::InvalidSerial { raw })
            );
        }
    }

    #[test]
    fn serial_and_entity_id_preserve_raw_order_and_hash_identity() {
        let low = Serial(1);
        let high = Serial(0x4000_0000);
        assert!(low < high);
        assert_eq!(HashSet::from([low, high, low]).len(), 2);

        let low_entity = EntityId::new(low.0).unwrap();
        let high_entity = EntityId::new(high.0).unwrap();
        assert_eq!(low_entity.serial(), low);
        assert_eq!(high_entity.serial(), high);
        assert!(low_entity < high_entity);
        assert_eq!(
            HashSet::from([low_entity, high_entity, low_entity]).len(),
            2
        );
    }

    #[test]
    fn point_and_map_values_round_trip_their_boundaries() {
        assert_eq!(Point3::new(i32::MIN, -1, i32::MAX).x, i32::MIN);
        assert_eq!(Point3::new(i32::MIN, -1, i32::MAX).y, -1);
        assert_eq!(Point3::new(i32::MIN, -1, i32::MAX).z, i32::MAX);
        for raw in [0, 1, u8::MAX] {
            assert_eq!(MapId::new(raw).raw(), raw);
        }
    }

    #[test]
    fn movement_direction_bytes_round_trip_and_reject_reserved_bits() {
        for (raw, expected) in [
            (0, Direction::North),
            (1, Direction::Right),
            (2, Direction::East),
            (3, Direction::Down),
            (4, Direction::South),
            (5, Direction::Left),
            (6, Direction::West),
            (7, Direction::Up),
        ] {
            for running in [false, true] {
                let wire = raw | if running { 0x80 } else { 0 };
                let decoded = Direction::from_raw(wire).unwrap();
                assert_eq!(decoded.base(), expected);
                assert_eq!(decoded.is_running(), running);
                assert_eq!(decoded.to_raw(), wire);
            }
        }
        for raw in [0x08, 0x40, 0x78, 0x88, 0xff] {
            assert_eq!(
                Direction::from_raw(raw),
                Err(CoreError::UnknownDirectionBits { raw })
            );
        }
    }

    #[test]
    fn all_eight_directions_expose_distinct_one_step_offsets() {
        for (direction, offset) in [
            (Direction::North, (0, -1)),
            (Direction::Right, (1, -1)),
            (Direction::East, (1, 0)),
            (Direction::Down, (1, 1)),
            (Direction::South, (0, 1)),
            (Direction::Left, (-1, 1)),
            (Direction::West, (-1, 0)),
            (Direction::Up, (-1, -1)),
        ] {
            assert_eq!(direction.offset(), offset);
            assert_eq!(
                Direction::from_raw(direction as u8).unwrap().offset(),
                offset
            );
        }
    }
}
