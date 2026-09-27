# Renaissance seeded world registry

## Boundary

Add an instance-owned `rustuo-world` entity registry with a deterministic
empty-map fixture and typed lookup/insertion failures. This slice does not
implement movement, terrain, packets, networking, or legacy changes.

## Legacy evidence

`legacy/Server/World.cs` uses serial-based mobile/entity lookup, while
`legacy/Server/Map.cs` rejects x/y outside map width and height. This Rust
fixture intentionally uses a separate local registry and map ID `0xFE`, not
Felucca terrain or live-client parity.

## Verification

- RED: `cargo test -p rustuo-world --lib` exited 101 before implementation
  with missing `WorldError`, map/lookup/insertion methods, and `EntityId`
  containment (24 expected compile errors).
- GREEN: focused world library tests passed (4/4): deterministic seed,
  half-open bounds, typed duplicate/not-found/out-of-bounds errors, and
  independent world instances.
- Workspace: `cargo fmt --all -- --check`, `cargo check --workspace`,
  `cargo test --workspace`, and `git diff --check` passed using a fresh
  `/home/opencode/.cache` Cargo target directory.
- Runtime harness: N/A — in-memory library registry with no runtime boundary.

Rollback boundary: revert this work-unit commit to remove only the world
registry, its tests, and this task record; the core values remain intact.
