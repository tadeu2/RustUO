# ODD Task: Renaissance Initial Self-Mobile Prefix

## Objective and evidence
Emit only the first pre-7.0.0 self `MobileIncoming` (`0x78`) and `MobileUpdateOld` (`0x20`) after map setup, as separate compressed frames. ServUO orders these in `legacy/Server/Network/PacketHandlers.cs:2530-2585`. Layouts come from `Packets.cs:3913-3936,4255-4402,5221-5226`; old flags from `Mobile.cs:8813-8853`; network byte order from `PacketWriter.cs:127-171`.

The seeded avatar supplies explicit hue `0x0456`, old flags `0x40`, and notoriety `1`. Notoriety is normally runtime-computed; these are fixture assumptions, not universal defaults. The `0x78` encoder supports **empty equipment only**, including a four-byte zero terminator. World supplies serial `1` and position `(3503, 2574, 14)`; avatar supplies body `0x0190` and direction `2`.

## TDD and validation
- RED: `cargo test -p rustuo-protocol --test renaissance_old_mobile` failed `E0432` for both absent encoders; `cargo test -p rustuo-server --test renaissance_world_entry_session initial_mobile_prefix_requires_map_setup_and_rejects_replay` failed `E0560`/`E0599` for absent presentation fields and method.
- GREEN: focused protocol test **3 passed**; focused server suite **7 passed**. Exact compressed vectors are literal values independently packed from `legacy/Server/Network/Compression.cs`, not generated with Rust's compressor. The tests also compare shared fields against the `0x1B` LoginConfirm layout.
- Final required checks (with `CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/home/opencode/.cache/rustuo-reconnect-huffman-target`): `cargo fmt --all -- --check` PASS; `cargo check --workspace` PASS; `cargo test --workspace` PASS; `git diff --check` PASS.
- Runtime harness: N/A; transport-neutral packet encoders and session composition only. No server/client/TCP run is authorized.

## Boundary and rollback
The session advances only after both compressed frames succeed; wrong phase and replay reject without mutation. Revert this work-unit commit to remove the two protocol encoders, seeded presentation fields, session phase/method, tests, and this record. It does not implement `SendEverything`, light updates, `0x55`, the later duplicate `0x78`, movement, TCP, or real-client acceptance. `legacy/` remains read-only.
