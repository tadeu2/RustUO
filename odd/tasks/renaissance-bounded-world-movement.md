# Renaissance bounded world movement

## Boundary

Add pure one-step movement decisions and generation-checked application to the
existing instance-local world fixture. Use only base direction offsets and
half-open `[0,16)` x/y bounds. Terrain, occupancy, speed policy, packet
acknowledgements, sessions, and legacy changes remain out of scope.

## Legacy evidence

`legacy/Server/Movement.cs` defines the eight direction offsets; the Rust
fixture applies those offsets without claiming the legacy terrain movement
policy. The OpenSpec world-domain slice requires a non-mutating decision and
typed, atomic application.

## Verification

- RED: `cargo test -p rustuo-world --lib` exited 101 with 27 expected compile
  errors for the missing decision type, movement methods, and stale error.
- GREEN: focused world library tests passed (10/10), including all eight
  offsets with and without running, corner diagonals, boundaries, replay,
  generation-only staleness after east-then-west, and error precedence.
- Workspace: `cargo fmt --all -- --check`, `cargo check --workspace`,
  `cargo test --workspace` (101 passed), and `git diff --check` passed using
  a fresh `/home/opencode/.cache` Cargo target directory.
- Runtime harness: N/A — pure in-memory world behavior, no transport boundary.

Rollback boundary: revert this work-unit commit to remove only flat movement,
its tests, and this task record; the seeded registry remains intact.
