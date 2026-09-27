# ODD Task: Renaissance Incremental Input Buffer

## Objective
Add a transport-independent owned buffer for packet bytes received across chunks. Emit complete frames without borrowing from mutable pending storage.

## Scope and boundary
- Implement `PacketInputBuffer` in `crates/rustuo-protocol/src/lib.rs` with only `new`, `append`, and `pending_len` as public methods.
- Compose the committed `decode_packet_frames` decoder. On success, return complete frames as owned `Vec<u8>` values and retain the exact incomplete suffix.
- On decode error, return no frames and leave the prior pending state unchanged. The rejected chunk is not retained; the caller owns any retry or connection policy.
- Do not add sockets, layout profiles, authentication, sessions, dispatch, `0xEF`, or a pending-size policy. Resource limits for untrusted ingress remain a future security and operations decision.
- Keep `legacy/` read-only and preserve `GPL-2.0-only` licensing.

## Delivery
A local conventional commit on `codex/legacy-account-bridge` is authorized. Do not push, create a PR, merge, or use remotes. Implement and validate in an isolated detached worktree from `5572939998f5173333f0bba6ae7d79a4a7421aaf`; do not cherry-pick into the dirty root. Advance the branch ref only after checks pass, without touching the root index or worktree. Limit the commit to this document and `crates/rustuo-protocol/src/lib.rs`, with at most 400 authored lines.

## Acceptance and evidence
- [x] Split fixed and variable frames emit only when complete.
- [x] Multiple complete frames remain ordered and owned; a partial suffix resumes exactly on the next append.
- [x] Empty chunks preserve state and return no frames.
- [x] Previously returned frames remain valid after future appends.
- [x] Unknown packet IDs and invalid variable lengths after a complete prefix roll back the entire append.
- [x] Strict TDD: focused RED failed with unresolved `PacketInputBuffer` import; seven focused tests passed after implementation.
- [x] Fresh-target `cargo test -p rustuo-protocol`: 41 passed, 0 failed.
- [x] `cargo fmt --all -- --check`: passed.
- [x] Fresh-target `cargo check --workspace`: passed.
- [x] Fresh-target `cargo test --workspace`: 83 tests passed, 0 failed (1 core, 41 protocol, 32 server unit, 3 compatibility, 6 XML-flow); doc tests had 0 tests.
- [x] `git diff --check`: passed.

Runtime validation: N/A for this pure protocol slice. The tests establish buffer behavior, not live network ingress, rate or memory limits, or end-to-end session behavior. The rollback boundary ends at `PacketInputBuffer` state; a caller must decide how to handle rejected bytes and connections.
