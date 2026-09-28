# ODD Task: Renaissance Server Selection Decoder

## Objective
Add a pure protocol decoder for the Renaissance `0xA0` server-selection frame.

## Problem
The protocol crate encodes the `0xA8` server list, but needs a typed boundary for the next client message: a three-byte selection packet carrying a signed server index. Session-dependent validation must not be hidden inside this parser.

## Why
The legacy handler reads one big-endian `Int16` before checking account/list state and creating the later `0x8C` acknowledgement. A pure decoder captures that wire contract while leaving authorization and connection setup to the session layer.

## Scope
- Add a pure `0xA0` decoder in `crates/rustuo-protocol/src/lib.rs`.
- Validate the exact three-byte frame shape and packet ID `0xA0`.
- Return the payload as signed big-endian `i16`, preserving negative values.
- Return explicit errors for wrong ID, truncation, and extra bytes.
- Add focused tests for positive and negative indexes, wrong ID, truncated frames, and extra bytes.

## Constraints
- Preserve all existing tracked and untracked worktree changes; do not normalize or remove unrelated work.
- Keep `legacy/` read-only.
- Do not validate the index against a server list, account, session, endpoint, or auth state.
- Do not create auth IDs, emit `0x8C`, dispatch handlers, add sockets, or implement session mutation.
- Keep the signed `i16` wire semantics; do not widen or reinterpret the value as an unsigned index.
- Keep generated technical artifacts in English.
- Strict TDD: RED -> GREEN -> REFACTOR.
- Focused test command: `cargo test -p rustuo-protocol server_selection_decoder`.
- Required checks: `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace`.
- Delivery: local work-unit commit on `codex/legacy-account-bridge`, using the user-selected `stacked-to-main` chain. No push, PR creation, remote, merge, deploy, or credential operation is included.

## Authorized scope
- Source: `crates/rustuo-protocol/src/lib.rs`
- Task tracking: this file and its Engram mirror.
- Verification may read the repository and run the commands above; do not modify unrelated source/config/OpenSpec paths.

## Checklist
- [x] SSD-1 — Add focused RED tests for the signed result and frame errors.
- [x] SSD-2 — Implement exact-frame ID/length validation and signed big-endian decoding.
- [x] SSD-3 — Run focused and required repository checks against the isolated final candidate and record evidence.

## Acceptance criteria
- A three-byte `0xA0` frame returns the signed big-endian `i16` index.
- Negative indexes are preserved for the eventual session validator; the parser does not authorize them.
- Wrong ID, truncated frame, and extra bytes return explicit errors without panicking.
- No server-list lookup, account validation, auth ID, `0x8C`, dispatch, session, or socket behavior is added.
- Focused and required repository checks report observed results.

## Verification Evidence
- TDD RED: the focused tests were added to a clean snapshot based on `f1a646e`; `cargo test -p rustuo-protocol server_selection_decoder` failed with exit 101 because the decoder API was absent (`E0432` unresolved imports), as expected.
- TDD GREEN: `cargo test -p rustuo-protocol server_selection_decoder` — 3 passed, 14 filtered.
- `cargo fmt --all -- --check` — passed.
- `cargo check --workspace` — passed.
- `cargo test --workspace` — passed: 30 unit tests total (core 1, protocol 17, server 12, world 0); doc-tests passed.
- Verification ran from an isolated clean snapshot containing the base commit plus only this slice; historical mixed-worktree counts are not used as evidence.
- Runtime harness: N/A — this pure protocol decoder has no runtime boundary.
- Rollback boundary: revert `feat(protocol): decode Renaissance server selection`; that removes only this decoder, its tests, and this task record.

## Progress
- Exploration confirmed `0xA0` is the next inbound message after the `0xA8` server list.
- Implementation: `decode_renaissance_server_selection` validates packet ID `0xA0` and exact three-byte length, returning `RenaissanceServerSelection { index: i16 }` with explicit wrong-ID, truncation, and extra-length errors.
- Residual boundary: the signed index remains unchecked against a server list or session; no `0x8C` response or connection behavior is included.
- Current status: implementation and isolated checks complete; local stacked work-unit commit is the delivery boundary. Commit subject: `feat(protocol): decode Renaissance server selection`.

## Next step
Continue with the next bounded protocol/session boundary only; keep server-list authorization and `0x8C` session behavior outside this decoder task.
