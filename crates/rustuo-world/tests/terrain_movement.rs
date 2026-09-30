use rustuo_core::{Direction, EntityId, Point3};
use rustuo_world::{TerrainTile, World, WorldError};

fn id(raw: u32) -> EntityId {
    EntityId::new(raw).unwrap()
}

fn east() -> rustuo_core::DecodedDirection {
    Direction::from_raw(2).unwrap()
}

#[test]
fn callers_can_configure_terrain_and_resolve_destination_ground_z() {
    let mut world = World::new();
    world
        .set_terrain_tile(9, 8, TerrainTile::walkable(2, 16))
        .unwrap();

    let decision = world.decide_move(id(1), east()).unwrap();
    world.apply_move(decision).unwrap();

    assert_eq!(
        world.lookup(id(1)).unwrap().position(),
        Point3::new(9, 8, 2)
    );
}

#[test]
fn terrain_configuration_rejects_coordinates_outside_the_world() {
    let mut world = World::new();

    assert_eq!(
        world.set_terrain_tile(16, 8, TerrainTile::walkable(0, 16)),
        Err(WorldError::TerrainOutOfBounds { x: 16, y: 8 })
    );
}

#[test]
fn impassable_terrain_rejects_movement_without_changing_position() {
    let mut world = World::new();
    world
        .set_terrain_tile(9, 8, TerrainTile::blocked(0, 16))
        .unwrap();

    let decision = world.decide_move(id(1), east()).unwrap();

    assert_eq!(
        world.apply_move(decision),
        Err(WorldError::TerrainBlocked {
            entity: id(1),
            target: Point3::new(9, 8, 0),
        })
    );
    assert_eq!(
        world.lookup(id(1)).unwrap().position(),
        Point3::new(8, 8, 0)
    );
}

#[test]
fn downward_step_requires_clearance_for_the_source_height_envelope() {
    let mut world = World::new();
    world.insert_entity(id(2), Point3::new(8, 8, 2)).unwrap();
    world
        .set_terrain_tile(9, 8, TerrainTile::walkable(0, 16))
        .unwrap();

    let insufficient = world.decide_move(id(2), east()).unwrap();
    assert_eq!(
        world.apply_move(insufficient),
        Err(WorldError::TerrainBlocked {
            entity: id(2),
            target: Point3::new(9, 8, 0),
        })
    );

    world
        .set_terrain_tile(9, 8, TerrainTile::walkable(0, 18))
        .unwrap();
    let sufficient = world.decide_move(id(2), east()).unwrap();
    world.apply_move(sufficient).unwrap();
    assert_eq!(
        world.lookup(id(2)).unwrap().position(),
        Point3::new(9, 8, 0)
    );
}

#[test]
fn terrain_change_invalidates_a_previously_decided_move() {
    let mut world = World::new();
    let stale = world.decide_move(id(1), east()).unwrap();
    world
        .set_terrain_tile(9, 8, TerrainTile::blocked(0, 16))
        .unwrap();

    assert_eq!(
        world.apply_move(stale),
        Err(WorldError::StaleDecision(id(1)))
    );
    assert_eq!(
        world.lookup(id(1)).unwrap().position(),
        Point3::new(8, 8, 0)
    );
}
