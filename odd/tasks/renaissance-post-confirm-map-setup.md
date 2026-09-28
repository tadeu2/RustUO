# ODD Task: Renaissance Post-Confirm Map Setup Prefix

## Objective
Emit the first three ServUO `DoLogin` packets after slot-zero `LoginConfirm` as distinct compressed frames, without starting transport or broader world entry.

## Contract and sources
- `legacy/Server/Network/PacketHandlers.cs:2530-2547` orders `LoginConfirm`, `MapChange` when map exists, `MapPatches`, then `SupportedFeatures` before mobile packets.
- `legacy/Server/Network/Packets.cs:258-282,3248-3258,3304-3345` defines raw map patch, map change, and feature formats; `legacy/Server/Network/PacketWriter.cs:127-171` defines network-order writes.
- `legacy/Server/TileMatrixPatch.cs:16-53` gets per-facet land/static counts from patch files. The seeded world has no patch store; four zero pairs are a fixture assumption ONLY, not a server-wide default.
- Protocol encoders take explicit map ID and four `(static_blocks, land_blocks)` pairs. Session takes its map ID from `World::map_id().raw()` and keeps the configured feature mask explicit.
- Wrong phase and replay reject without mutation; state advances only after all three frames are encoded and compressed.

## TDD and validation
- RED: `cargo test -p rustuo-protocol --test renaissance_map_setup` failed `E0432` for absent encoders; `cargo test -p rustuo-server --test renaissance_world_entry_session map_setup_follows_confirm_once_and_preserves_frame_boundaries` failed `E0599` for absent session method.
- GREEN: focused protocol test `2 passed`; focused session test `6 passed`. Compressed expectations are independent literal vectors derived from the legacy Huffman table, not the Rust compressor.
- Final focused commands: `cargo test -p rustuo-protocol --test renaissance_map_setup`; `cargo test -p rustuo-server --test renaissance_world_entry_session`.
- Full required commands: `cargo fmt --all -- --check` PASS; `cargo check --workspace` PASS; `cargo test --workspace` PASS (119 tests); `git diff --check` PASS, all with `CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/home/opencode/.cache/rustuo-reconnect-huffman-target` where applicable.
- Runtime harness: N/A. This is transport-neutral packet composition; no TCP/client/server runtime is authorized.

## Boundary and rollback
Revert this one work-unit commit to remove only the protocol map encoders, post-confirm session phase/method, their tests, and this record. Do not include `MobileIncoming`, `Update`, `SendEverything`, `LoginComplete`/`0x55`, TCP, or real-client acceptance claims. `legacy/` remains read-only.
