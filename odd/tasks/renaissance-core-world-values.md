# Renaissance core world values

## Boundary

Add pure `rustuo-core` values for later seeded-world and movement slices: validated
`EntityId`, signed `Point3`, opaque `MapId`, and eight-way `Direction` with a
decoded movement byte. No world fixture, terrain, networking, or legacy edits.

## Legacy evidence

`legacy/Server/Serial.cs` separates mobile serials below `0x40000000` from
item serials through `0x7FFFFFFF`. `legacy/Server/Mobile.cs` defines eight
directions, a `0x07` direction mask, and a `0x80` running flag. The Rust decoder
rejects all remaining bits rather than silently discarding them.

## Verification

- RED: `cargo test -p rustuo-core --lib` failed on missing domain symbols and
  error variants before production code was added.
- GREEN: `cargo test -p rustuo-core --lib` passed 5 tests, including boundary,
  round-trip, reserved-bit, and eight-offset coverage.
- Workspace: `cargo fmt --all -- --check`, `cargo check --workspace`, and
  `cargo test --workspace` passed with a fresh `CARGO_TARGET_DIR` outside full
  `/tmp`; `git diff --check` passed. Workspace tests: 89 passed, 0 failed.
- Runtime harness: N/A — pure value types.

Rollback boundary: revert this work-unit commit to remove the core values and
their tests plus this task record; no later world or transport behavior is part
of this commit.
