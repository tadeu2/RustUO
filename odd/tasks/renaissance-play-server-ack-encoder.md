# ODD Task: Renaissance Play Server Ack Encoder

## Objective
Add a pure encoder for the fixed Renaissance `0x8C` PlayServer acknowledgement frame.

## Problem
The protocol crate parses the `0xA0` server selection, but needs a codec boundary for the deterministic response legacy emits after session validation. The encoder must accept already-prepared values without generating auth state or mutating a connection.

## Why
Legacy `PlayServer` performs account/list/range validation, creates an auth ID, and then serializes an 11-byte packet. That serialization is independently testable and remains separate from session integration.

## Scope
- Add a pure `0x8C` encoder in `crates/rustuo-protocol/src/lib.rs` from prepared raw address, port, and auth ID values.
- Emit packet ID `0x8C`, address `u32` low-byte-first, port `u16` big-endian, and auth ID `u32` big-endian.
- Keep address and auth ID as raw wire values; do not parse IPs, generate auth IDs, or validate endpoints.
- Add a focused exact-byte test for packet ID/length, address byte order, port, and auth ID.

## Constraints
- Preserve all existing tracked and untracked worktree changes; do not normalize or remove unrelated work.
- Keep `legacy/` read-only.
- Do not validate selection/account/list state, create auth IDs, mutate sessions, dispatch handlers, open sockets, send packets, or implement connection handoff.
- Do not reuse the `0xA8` address serialization: `0x8C` intentionally writes the address low-byte-first.
- Keep technical artifacts in English.
- Strict TDD: RED -> GREEN -> REFACTOR.
- Focused test command: `cargo test -p rustuo-protocol play_server_ack_encoder`.
- Required checks: `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace`.
- Delivery: local work-unit commit on `codex/legacy-account-bridge`, using the user-selected `stacked-to-main` chain. No push, PR creation, remote, merge, deploy, or credential operation is included.

## Authorized scope
- Source: `crates/rustuo-protocol/src/lib.rs`
- Task tracking: this file and its Engram mirror.
- Verification may read the repository and run the commands above; do not modify unrelated source/config/OpenSpec paths.

## Checklist
- [x] PSA-1 — Add the exact-byte RED test for the 11-byte wire layout.
- [x] PSA-2 — Implement fixed-frame serialization with the legacy field byte orders.
- [x] PSA-3 — Run focused and required repository checks against the isolated final candidate and record evidence.

## Acceptance criteria
- The encoder returns exactly 11 bytes beginning with `0x8C`.
- Address bytes are low-byte-first; port and auth ID are big-endian.
- Prepared raw values are serialized without IP/auth interpretation or validation.
- No account/session/auth generation, dispatch, socket, sending, or connection-handoff behavior is added.
- Focused and required repository checks report observed results.

## Verification Evidence
- TDD RED: the focused test was added to a clean snapshot based on `9a6e2e7`; `cargo test -p rustuo-protocol play_server_ack_encoder` failed with exit 101 because `encode_renaissance_play_server_ack` was absent (`E0432` unresolved import), as expected.
- TDD GREEN: `cargo test -p rustuo-protocol play_server_ack_encoder` — 1 passed, 17 filtered.
- `cargo fmt --all -- --check` — passed.
- `cargo check --workspace` — passed.
- `cargo test --workspace` — passed: 31 unit tests total (core 1, protocol 18, server 12, world 0); doc-tests passed.
- Verification ran from an isolated clean snapshot containing the base commit plus only this slice; historical mixed-worktree counts are not used as evidence.
- Runtime harness: N/A — this pure protocol encoder has no runtime boundary.
- Rollback boundary: revert `feat(protocol): encode Renaissance play server ack`; that removes only this encoder, its test, and this task record.

## Progress
- Exploration confirmed `0x8C` is the fixed response codec after `0xA0` server selection; session validation and auth generation remain outside the encoder.
- Implementation: `encode_renaissance_play_server_ack(address, port, auth_id)` writes packet ID `0x8C`, address little-endian bytes, then big-endian port and auth ID using `PacketWriter`.
- Residual boundary: this codec does not parse or validate address/auth values and does not perform session validation, auth generation, dispatch, sending, or connection handoff.
- Current status: implementation and isolated checks complete; local stacked work-unit commit is the delivery boundary. Commit subject: `feat(protocol): encode Renaissance play server ack`.

## Next step
Continue with the next independent pure codec boundary, PRE-GLD, before the deferred session-selection/reconnect integration slices.
