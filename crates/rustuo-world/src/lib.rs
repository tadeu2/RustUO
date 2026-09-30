#![forbid(unsafe_code)]

//! World model and persistence boundary.

use std::collections::HashMap;

use rustuo_core::{DecodedDirection, EntityId, MapId, Point3};

const FIXTURE_MAP: MapId = MapId::new(0xfe);
const FIXTURE_SIZE: i32 = 16;
const STEP_HEIGHT: i32 = 2;
const PERSON_HEIGHT: i32 = 16;
const TRAMMEL_MAP: MapId = MapId::new(1);
const TRAMMEL_SIZE: (i32, i32) = (7168, 4096);
const NEW_HAVEN_BANK: Point3 = Point3::new(3503, 2574, 14);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entity {
    id: EntityId,
    map: MapId,
    position: Point3,
    generation: u64,
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
    TerrainOutOfBounds { x: i32, y: i32 },
    TerrainBlocked { entity: EntityId, target: Point3 },
    StaleDecision(EntityId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MoveState {
    Accepted,
    RejectedOutOfBounds,
    RejectedTerrain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainTile {
    ground_z: i32,
    headroom: i32,
    impassable: bool,
}

impl TerrainTile {
    /// Creates a walkable terrain cell with the supplied ground height and clearance.
    pub const fn walkable(ground_z: i32, headroom: i32) -> Self {
        Self {
            ground_z,
            headroom,
            impassable: false,
        }
    }

    /// Creates an impassable terrain cell with the supplied ground height and clearance.
    pub const fn blocked(ground_z: i32, headroom: i32) -> Self {
        Self {
            ground_z,
            headroom,
            impassable: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveDecision {
    entity: EntityId,
    source: Point3,
    source_generation: u64,
    terrain_generation: u64,
    target: Point3,
    state: MoveState,
}

#[derive(Debug)]
pub struct World {
    entities: HashMap<EntityId, Entity>,
    player_id: EntityId,
    map_id: MapId,
    map_size: (i32, i32),
    terrain: HashMap<(i32, i32), TerrainTile>,
    terrain_generation: u64,
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

impl World {
    pub fn new() -> Self {
        Self::seeded(
            FIXTURE_MAP,
            (FIXTURE_SIZE, FIXTURE_SIZE),
            Point3::new(8, 8, 0),
        )
    }

    pub fn renaissance_client_fixture() -> Self {
        Self::seeded(TRAMMEL_MAP, TRAMMEL_SIZE, NEW_HAVEN_BANK)
    }

    fn seeded(map_id: MapId, map_size: (i32, i32), player_position: Point3) -> Self {
        let player_id = EntityId::new(1).expect("fixture player ID is valid");
        let mut world = Self {
            entities: HashMap::new(),
            player_id,
            map_id,
            map_size,
            terrain: HashMap::new(),
            terrain_generation: 0,
        };
        world
            .insert_entity(player_id, player_position)
            .expect("fixture seed is valid");
        world
    }

    pub const fn map_id(&self) -> MapId {
        self.map_id
    }

    pub const fn map_size(&self) -> (i32, i32) {
        self.map_size
    }

    /// Sets or replaces the in-memory terrain sample used by movement decisions.
    pub fn set_terrain_tile(
        &mut self,
        x: i32,
        y: i32,
        tile: TerrainTile,
    ) -> Result<(), WorldError> {
        if !self.in_bounds(Point3::new(x, y, tile.ground_z)) {
            return Err(WorldError::TerrainOutOfBounds { x, y });
        }
        self.terrain.insert((x, y), tile);
        self.terrain_generation += 1;
        Ok(())
    }

    pub const fn in_bounds(&self, point: Point3) -> bool {
        point.x >= 0 && point.x < self.map_size.0 && point.y >= 0 && point.y < self.map_size.1
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
                map: self.map_id,
                position,
                generation: 0,
            },
        );
        Ok(())
    }

    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    pub fn decide_move(
        &self,
        id: EntityId,
        direction: DecodedDirection,
    ) -> Result<MoveDecision, WorldError> {
        let entity = self.lookup(id)?;
        let (dx, dy) = direction.offset();
        let source = entity.position;
        let mut target = Point3::new(source.x + dx, source.y + dy, source.z);
        let mut state = MoveState::Accepted;
        if !self.in_bounds(target) {
            state = MoveState::RejectedOutOfBounds;
        } else if let Some(ground_z) = self.passable_ground_z(target.x, target.y, source.z) {
            target.z = ground_z;
            if id == self.player_id && dx != 0 && dy != 0 {
                let side_x = self.passable_ground_z(source.x + dx, source.y, source.z);
                let side_y = self.passable_ground_z(source.x, source.y + dy, source.z);
                if side_x.is_none() || side_y.is_none() {
                    state = MoveState::RejectedTerrain;
                }
            }
        } else {
            if let Some(tile) = self.terrain.get(&(target.x, target.y)) {
                target.z = tile.ground_z;
            }
            state = MoveState::RejectedTerrain;
        }
        Ok(MoveDecision {
            entity: id,
            source,
            source_generation: entity.generation,
            terrain_generation: self.terrain_generation,
            target,
            state,
        })
    }

    fn passable_ground_z(&self, x: i32, y: i32, source_z: i32) -> Option<i32> {
        let Some(tile) = self.terrain.get(&(x, y)) else {
            return Some(source_z);
        };
        let destination_z = i64::from(tile.ground_z);
        let source_z = i64::from(source_z);
        let required_headroom =
            source_z.max(destination_z) + i64::from(PERSON_HEIGHT) - destination_z;
        (!tile.impassable
            && destination_z <= source_z + i64::from(STEP_HEIGHT)
            && i64::from(tile.headroom) >= required_headroom)
            .then_some(tile.ground_z)
    }

    pub fn apply_move(&mut self, decision: MoveDecision) -> Result<(), WorldError> {
        let terrain_generation = self.terrain_generation;
        let entity = self
            .entities
            .get_mut(&decision.entity)
            .ok_or(WorldError::NotFound(decision.entity))?;
        if entity.position != decision.source
            || entity.generation != decision.source_generation
            || terrain_generation != decision.terrain_generation
        {
            return Err(WorldError::StaleDecision(decision.entity));
        }
        if decision.state == MoveState::RejectedOutOfBounds {
            return Err(WorldError::OutOfBounds {
                entity: decision.entity,
                target: decision.target,
            });
        }
        if decision.state == MoveState::RejectedTerrain {
            return Err(WorldError::TerrainBlocked {
                entity: decision.entity,
                target: decision.target,
            });
        }
        entity.position = decision.target;
        entity.generation += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{MoveState, World, WorldError};
    use rustuo_core::{Direction, EntityId, MapId, Point3};

    fn id(raw: u32) -> EntityId {
        EntityId::new(raw).unwrap()
    }

    fn direction(raw: u8) -> rustuo_core::DecodedDirection {
        Direction::from_raw(raw).unwrap()
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
    fn renaissance_client_fixture_uses_trammel_new_haven_and_accepts_eastward_step() {
        let mut world = World::renaissance_client_fixture();
        let player = id(1);
        assert_eq!(world.map_id(), MapId::new(1));
        assert_eq!(world.map_size(), (7168, 4096));
        assert!(world.in_bounds(Point3::new(0, 0, 0)));
        assert!(world.in_bounds(Point3::new(7167, 4095, 0)));
        assert!(!world.in_bounds(Point3::new(7168, 4095, 0)));
        assert!(!world.in_bounds(Point3::new(7167, 4096, 0)));
        assert_eq!(world.entity_count(), 1);
        assert_eq!(world.player().id(), player);
        assert_eq!(world.player().map(), MapId::new(1));
        assert_eq!(world.player().position(), Point3::new(3503, 2574, 14));

        let east = world.decide_move(player, direction(2)).unwrap();
        world.apply_move(east).unwrap();
        assert_eq!(world.player().position(), Point3::new(3504, 2574, 14));
        assert_eq!(world.player().map(), MapId::new(1));
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

    #[test]
    fn decisions_are_pure_and_all_eight_offsets_preserve_z_and_ownership() {
        for raw in 0..8 {
            for running in [false, true] {
                let mut world = World::new();
                let before = *world.player();
                let decoded = direction(raw | if running { 0x80 } else { 0 });
                let (dx, dy) = decoded.offset();
                let decision = world.decide_move(id(1), decoded).unwrap();
                assert_eq!(decision.entity, id(1));
                assert_eq!(decision.source, before.position());
                assert_eq!(decision.source_generation, 0);
                assert_eq!(decision.target, Point3::new(8 + dx, 8 + dy, 0));
                assert_eq!(decision.state, MoveState::Accepted);
                assert_eq!(*world.player(), before);

                world.apply_move(decision).unwrap();
                assert_eq!(world.player().position(), decision.target);
                assert_eq!(world.player().map(), before.map());
                assert_eq!(world.entity_count(), 1);
                assert_eq!(world.lookup(id(1)).unwrap(), world.player());
                assert_eq!(
                    world.decide_move(id(1), decoded).unwrap().source_generation,
                    1
                );
            }
        }
    }

    #[test]
    fn corner_diagonals_are_accepted_without_side_tile_policy() {
        for (start, raw, target) in [
            (Point3::new(0, 0, 7), 3, Point3::new(1, 1, 7)),
            (Point3::new(15, 0, 7), 5, Point3::new(14, 1, 7)),
            (Point3::new(15, 15, 7), 7, Point3::new(14, 14, 7)),
            (Point3::new(0, 15, 7), 1, Point3::new(1, 14, 7)),
        ] {
            let mut world = World::new();
            world.insert_entity(id(2), start).unwrap();
            let decision = world.decide_move(id(2), direction(raw)).unwrap();
            assert_eq!(decision.state, MoveState::Accepted);
            world.apply_move(decision).unwrap();
            assert_eq!(world.lookup(id(2)).unwrap().position(), target);
            assert_eq!(world.player().position(), Point3::new(8, 8, 0));
        }
    }

    #[test]
    fn rejected_boundaries_and_unknown_entity_do_not_mutate() {
        let mut world = World::new();
        assert_eq!(
            world.decide_move(id(2), direction(2)),
            Err(WorldError::NotFound(id(2)))
        );
        for (start, raw, target) in [
            (Point3::new(0, 0, -4), 7, Point3::new(-1, -1, -4)),
            (Point3::new(15, 15, 9), 3, Point3::new(16, 16, 9)),
        ] {
            world.insert_entity(id(2), start).unwrap();
            let decision = world.decide_move(id(2), direction(raw)).unwrap();
            assert_eq!(decision.state, MoveState::RejectedOutOfBounds);
            assert_eq!(decision.target, target);
            assert_eq!(world.lookup(id(2)).unwrap().position(), start);
            assert_eq!(
                world.apply_move(decision),
                Err(WorldError::OutOfBounds {
                    entity: id(2),
                    target
                })
            );
            assert_eq!(world.lookup(id(2)).unwrap().position(), start);
            assert_eq!(world.lookup(id(2)).unwrap().map(), world.map_id());
            assert_eq!(world.entity_count(), 2);
            // Reuse the same fixture record after checking both extreme boundaries.
            world.entities.remove(&id(2));
        }
    }

    #[test]
    fn stale_and_replayed_decisions_fail_even_after_return_to_source() {
        let mut world = World::new();
        let east = world.decide_move(id(1), direction(2)).unwrap();
        let other = world.decide_move(id(1), direction(0)).unwrap();
        world.apply_move(east).unwrap();
        assert_eq!(
            world.apply_move(east),
            Err(WorldError::StaleDecision(id(1)))
        );
        assert_eq!(
            world.apply_move(other),
            Err(WorldError::StaleDecision(id(1)))
        );
        let west = world.decide_move(id(1), direction(6)).unwrap();
        world.apply_move(west).unwrap();
        assert_eq!(world.player().position(), east.source);
        assert_eq!(
            world.apply_move(east),
            Err(WorldError::StaleDecision(id(1)))
        );
        assert_eq!(world.player().position(), east.source);
        assert_eq!(
            world
                .decide_move(id(1), direction(2))
                .unwrap()
                .source_generation,
            2
        );
    }

    #[test]
    fn stale_precedes_rejected_state_and_unknown_precedes_stale() {
        let mut world = World::new();
        world.insert_entity(id(2), Point3::new(15, 15, 0)).unwrap();
        let rejected = world.decide_move(id(2), direction(3)).unwrap();
        let west = world.decide_move(id(2), direction(6)).unwrap();
        world.apply_move(west).unwrap();
        assert_eq!(
            world.apply_move(rejected),
            Err(WorldError::StaleDecision(id(2)))
        );
        world.entities.remove(&id(2));
        assert_eq!(world.apply_move(rejected), Err(WorldError::NotFound(id(2))));
    }

    #[test]
    fn terrain_move_resolves_destination_ground_z() {
        let mut world = World::new();
        world
            .set_terrain_tile(9, 8, super::TerrainTile::walkable(2, 16))
            .unwrap();

        let decision = world.decide_move(id(1), direction(2)).unwrap();

        assert_eq!(decision.target, Point3::new(9, 8, 2));
        world.apply_move(decision).unwrap();
        assert_eq!(world.player().position(), Point3::new(9, 8, 2));

        world
            .set_terrain_tile(8, 8, super::TerrainTile::walkable(-100, 118))
            .unwrap();
        let downward = world.decide_move(id(1), direction(6)).unwrap();
        assert_eq!(downward.target, Point3::new(8, 8, -100));
        world.apply_move(downward).unwrap();
        assert_eq!(world.player().position(), Point3::new(8, 8, -100));
    }

    #[test]
    fn terrain_over_step_rejection_does_not_mutate_entity() {
        let mut world = World::new();
        world
            .set_terrain_tile(9, 8, super::TerrainTile::walkable(3, 16))
            .unwrap();

        let decision = world.decide_move(id(1), direction(2)).unwrap();

        assert_eq!(decision.state, MoveState::RejectedTerrain);
        assert_eq!(
            world.apply_move(decision),
            Err(WorldError::TerrainBlocked {
                entity: id(1),
                target: Point3::new(9, 8, 3),
            })
        );
        assert_eq!(world.player().position(), Point3::new(8, 8, 0));
    }

    #[test]
    fn terrain_headroom_accepts_exact_person_height_and_rejects_less() {
        let mut world = World::new();
        world
            .set_terrain_tile(9, 8, super::TerrainTile::walkable(0, 15))
            .unwrap();
        let blocked = world.decide_move(id(1), direction(2)).unwrap();
        assert_eq!(blocked.state, MoveState::RejectedTerrain);

        world
            .set_terrain_tile(9, 8, super::TerrainTile::walkable(0, 16))
            .unwrap();
        let clear = world.decide_move(id(1), direction(2)).unwrap();
        assert_eq!(clear.state, MoveState::Accepted);
    }

    #[test]
    fn player_diagonal_requires_both_side_cells_to_be_passable() {
        let mut world = World::new();
        world
            .set_terrain_tile(9, 8, super::TerrainTile::walkable(3, 16))
            .unwrap();
        world
            .set_terrain_tile(8, 9, super::TerrainTile::walkable(0, 16))
            .unwrap();
        let blocked_side = world.decide_move(id(1), direction(3)).unwrap();
        assert_eq!(blocked_side.state, MoveState::RejectedTerrain);

        world
            .set_terrain_tile(9, 8, super::TerrainTile::walkable(2, 16))
            .unwrap();
        let passable_sides = world.decide_move(id(1), direction(3)).unwrap();
        assert_eq!(passable_sides.state, MoveState::Accepted);
    }

    #[test]
    fn terrain_move_invalidates_previously_decided_move() {
        let mut world = World::new();
        world
            .set_terrain_tile(9, 8, super::TerrainTile::walkable(2, 16))
            .unwrap();
        let terrain_move = world.decide_move(id(1), direction(2)).unwrap();
        let stale = world.decide_move(id(1), direction(0)).unwrap();
        world.apply_move(terrain_move).unwrap();

        assert_eq!(
            world.apply_move(stale),
            Err(WorldError::StaleDecision(id(1)))
        );
        assert_eq!(world.player().position(), Point3::new(9, 8, 2));
    }
}
