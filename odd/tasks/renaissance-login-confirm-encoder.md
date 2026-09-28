# ODD Task: Renaissance LoginConfirm Encoder

## Objective
Encode ServUO's fixed `0x1B` LoginConfirm frame from prepared world-entry fields.

## Legacy contract
`legacy/Server/Network/Packets.cs` constructs a 37-byte frame: packet ID; big-endian serial; zero `u32`; big-endian body, x, y, signed z; direction; zero byte; all-ones `u32`; two zero `u16` fields; big-endian map width and height; then six zero fill bytes. The dimensions are resolved by the caller; no map ID is serialized.

## Scope and boundary
- Add a pure encoder and exact 37-byte fixture test in `crates/rustuo-protocol/src/lib.rs`.
- Accept already-resolved packet fields; do not inspect maps, mobiles, sessions, or accounts.
- Do not send packets or claim full world entry or real-client compatibility.
- Keep `legacy/` read-only and leave unrelated WIP untouched.

## Acceptance evidence
- RED: A test-only change failed `cargo test -p rustuo-protocol login_confirm_encoder` with `E0432` because `encode_renaissance_login_confirm` was absent.
- GREEN: The focused test passed after implementation and a hand-fixture count correction; it asserts the full 37-byte frame and negative z (`-2` as `FF FE`).
- `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace`, and `git diff --check` passed. The workspace suite ran 107 unit/integration tests plus passing doc-tests.

## Next boundary
The packet is only one world-entry serialization step. Session selection, map resolution, remaining world-entry packets, network delivery, and real-client testing remain separate work.
