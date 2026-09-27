#![forbid(unsafe_code)]

//! World model and persistence boundary.

use std::collections::HashMap;

use rustuo_core::{EntityId, MapId, Point3};

const FIXTURE_MAP: MapId = MapId::new(0xfe);
const FIXTURE_SIZE: i32 = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entity {
    id: EntityId,
    map: MapId,
    position: Point3,
}

impl Entity {
    pub const fn id(&self) -> EntityId {
        self.id
    }

    pub const fn map(&self) -> MapId {
        self.map
    }

    pub const fn position(&self) -> Point3 {
        self.position
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WorldError {
    NotFound(EntityId),
    Duplicate(EntityId),
    OutOfBounds { entity: EntityId, target: Point3 },
}

#[derive(Debug)]
pub struct World {
    entities: HashMap<EntityId, Entity>,
    player_id: EntityId,
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    pub fn new() -> Self {
        let player_id = EntityId::new(1).expect("fixture player ID is valid");
        let mut world = Self {
            entities: HashMap::new(),
            player_id,
        };
        world
            .insert_entity(player_id, Point3::new(8, 8, 0))
            .expect("fixture seed is valid");
        world
    }

    pub const fn map_id(&self) -> MapId {
        FIXTURE_MAP
    }

    pub const fn map_size(&self) -> (i32, i32) {
        (FIXTURE_SIZE, FIXTURE_SIZE)
    }

    pub const fn in_bounds(&self, point: Point3) -> bool {
        point.x >= 0 && point.x < FIXTURE_SIZE && point.y >= 0 && point.y < FIXTURE_SIZE
    }

    pub fn contains(&self, id: EntityId) -> bool {
        self.entities.contains_key(&id)
    }

    pub fn lookup(&self, id: EntityId) -> Result<&Entity, WorldError> {
        self.entities.get(&id).ok_or(WorldError::NotFound(id))
    }

    pub fn player(&self) -> &Entity {
        self.entities
            .get(&self.player_id)
            .expect("fixture player is always present")
    }

    pub fn insert_entity(&mut self, id: EntityId, position: Point3) -> Result<(), WorldError> {
        if self.contains(id) {
            return Err(WorldError::Duplicate(id));
        }
        if !self.in_bounds(position) {
            return Err(WorldError::OutOfBounds {
                entity: id,
                target: position,
            });
        }
        self.entities.insert(
            id,
            Entity {
                id,
                map: FIXTURE_MAP,
                position,
            },
        );
        Ok(())
    }

    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }
}

#[cfg(test)]
mod tests {
    use super::{World, WorldError};
    use rustuo_core::{EntityId, MapId, Point3};

    fn id(raw: u32) -> EntityId {
        EntityId::new(raw).unwrap()
    }

    #[test]
    fn new_world_has_deterministic_empty_map_and_player_seed() {
        let first = World::new();
        let second = World::new();
        for world in [&first, &second] {
            assert_eq!(world.map_id(), MapId::new(0xfe));
            assert_eq!(world.map_size(), (16, 16));
            assert!(world.in_bounds(Point3::new(0, 0, i32::MIN)));
            assert!(world.in_bounds(Point3::new(15, 15, i32::MAX)));
            for point in [
                Point3::new(-1, 0, 0),
                Point3::new(0, -1, 0),
                Point3::new(16, 0, 0),
                Point3::new(0, 16, 0),
            ] {
                assert!(!world.in_bounds(point));
            }
            assert_eq!(world.entity_count(), 1);
            assert!(world.contains(id(1)));
            let entity = world.lookup(id(1)).unwrap();
            assert_eq!(entity.id(), id(1));
            assert_eq!(entity.map(), MapId::new(0xfe));
            assert_eq!(entity.position(), Point3::new(8, 8, 0));
            assert_eq!(world.player(), entity);
        }
    }

    #[test]
    fn lookup_is_typed_and_duplicate_insertion_preserves_original() {
        let mut world = World::new();
        assert_eq!(world.lookup(id(2)), Err(WorldError::NotFound(id(2))));
        assert!(!world.contains(id(2)));

        assert_eq!(
            world.insert_entity(id(1), Point3::new(3, 4, 5)),
            Err(WorldError::Duplicate(id(1)))
        );
        assert_eq!(world.entity_count(), 1);
        assert_eq!(world.player().position(), Point3::new(8, 8, 0));
    }

    #[test]
    fn insertion_validates_half_open_bounds_without_mutation() {
        let mut world = World::new();
        for (raw, point) in [
            (2, Point3::new(-1, 0, 0)),
            (3, Point3::new(0, -1, 0)),
            (4, Point3::new(16, 0, 0)),
            (5, Point3::new(0, 16, 0)),
        ] {
            assert_eq!(
                world.insert_entity(id(raw), point),
                Err(WorldError::OutOfBounds {
                    entity: id(raw),
                    target: point,
                })
            );
            assert!(!world.contains(id(raw)));
            assert_eq!(world.entity_count(), 1);
        }
        world.insert_entity(id(2), Point3::new(15, 15, -3)).unwrap();
        assert_eq!(
            world.lookup(id(2)).unwrap().position(),
            Point3::new(15, 15, -3)
        );
        assert_eq!(world.entity_count(), 2);
    }

    #[test]
    fn worlds_own_independent_registries_even_with_identical_ids() {
        let mut first = World::new();
        let mut second = World::new();
        first.insert_entity(id(2), Point3::new(0, 0, 0)).unwrap();
        second.insert_entity(id(2), Point3::new(15, 15, 7)).unwrap();
        assert_eq!(
            first.lookup(id(2)).unwrap().position(),
            Point3::new(0, 0, 0)
        );
        assert_eq!(
            second.lookup(id(2)).unwrap().position(),
            Point3::new(15, 15, 7)
        );
        assert_eq!(first.entity_count(), 2);
        assert_eq!(second.entity_count(), 2);
    }

    #[test]
    fn item_entity_lookup_does_not_change_fixture_player() {
        let mut world = World::new();
        let item = id(0x4000_0000);
        world.insert_entity(item, Point3::new(3, 4, 5)).unwrap();

        assert!(world.contains(item));
        assert_eq!(world.lookup(item).unwrap().id(), item);
        assert_eq!(world.lookup(item).unwrap().position(), Point3::new(3, 4, 5));
        assert_eq!(world.player().id(), id(1));
        assert_eq!(world.player().position(), Point3::new(8, 8, 0));
        assert_eq!(world.entity_count(), 2);
    }
}
