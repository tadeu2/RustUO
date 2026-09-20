#![forbid(unsafe_code)]

//! World model and persistence boundary.

use rustuo_core::Serial;

#[derive(Debug, Default)]
pub struct World {
    entity_count: usize,
}

impl World {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn contains(&self, _serial: Serial) -> bool {
        false
    }

    pub fn entity_count(&self) -> usize {
        self.entity_count
    }
}
