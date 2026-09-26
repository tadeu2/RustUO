# ODD Task: Renaissance Game Login Decoder

## Objective
Add a pure decoder for the fixed Renaissance `0x91` game-login frame.

## Problem
The protocol crate now decodes account login and server selection and encodes the PlayServer acknowledgement, but needs a typed wire boundary for the later game-login frame carrying an auth ID and two fixed-width credential fields.

## Why
The application layer needs a precise raw packet representation before deciding how to consume a reconnect grant or apply credential/session policy. Keeping this decoder pure avoids leaking sensitive field semantics into the protocol crate.

## Scope
- Add `decode_renaissance_game_login` in `crates/rustuo-protocol/src/lib.rs` for the exact 65-byte frame.
- Require packet ID `0x91`; decode auth ID as big-endian `u32` from bytes `1..5`.
- Return borrowed raw username and password slices from the fixed 30-byte fields at bytes `5..35` and `35..65`, truncated at the first NUL while consuming their full widths.
- Add focused tests for byte order, field offsets and borrowing, NUL truncation, raw non-UTF-8 bytes, zero/high-bit auth IDs, and exact-length/packet-ID errors.

## Constraints
- Preserve all existing tracked and untracked worktree changes; do not normalize or remove unrelated work.
- Keep `legacy/` read-only.
- Do not consume reconnect grants, validate credentials, authenticate accounts, apply client-verification fallback, check seed/session state, dispatch packets, open sockets, accept reconnects, or perform live handoff.
- Keep credential fields borrowed and raw; do not decode UTF-8, log, copy, trim whitespace, or apply policy.
- Keep application/grant ownership in `rustuo-server`; this crate only parses bytes.
- Keep technical artifacts in English.
- Strict TDD: RED -> GREEN -> REFACTOR.
- Focused test command: `cargo test -p rustuo-protocol game_login_decoder`.
- Required checks: `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace`.
- Delivery: local work-unit commit on `codex/legacy-account-bridge`, using the user-selected `stacked-to-main` chain. No push, PR creation, remote, merge, deploy, or credential operation is included.

## Authorized scope
- Source: `crates/rustuo-protocol/src/lib.rs`.
- Task tracking: this file and its Engram mirror.
- Verification may read the repository and run the commands above; do not modify unrelated source/config/OpenSpec paths.

## Checklist
- [x] GLD-1 — Add focused RED tests for the raw borrowed fields and exact-frame errors.
- [x] GLD-2 — Implement exact ID/length validation, big-endian auth ID, and fixed-width NUL-truncated slices.
- [x] GLD-3 — Run focused and required repository checks against the isolated final candidate and record evidence.

## Acceptance criteria
- The decoder accepts only exactly 65 bytes beginning with `0x91`.
- Auth ID is decoded big-endian; username/password are borrowed raw slices at the legacy offsets.
- NUL truncation does not alter field offsets or copy sensitive bytes.
- No grant consumption, credential/session policy, transport, dispatch, socket, reconnect acceptance, or handoff behavior is added.
- Focused and required repository checks report observed results.

## Verification Evidence
- TDD RED: focused tests were added to a clean snapshot based on `58467a5`; `cargo test -p rustuo-protocol game_login_decoder` failed with exit 101 because the decoder API was absent (`E0432` unresolved imports), as expected.
- TDD GREEN: `cargo test -p rustuo-protocol game_login_decoder` — 4 passed, 18 filtered.
- `cargo fmt --all -- --check` — passed.
- `cargo check --workspace` — passed.
- `cargo test --workspace` — passed: 35 unit tests total (core 1, protocol 22, server 12, world 0); doc-tests passed.
- Verification ran from an isolated clean snapshot containing the base commit plus only this slice; historical mixed-worktree counts are not used as evidence.
- Runtime harness: N/A — this pure protocol decoder has no runtime boundary.
- Rollback boundary: revert `feat(protocol): decode Renaissance game login`; that removes only this decoder, its tests, and this task record.

## Progress
- Exploration confirmed the legacy `0x91` frame is a fixed 65-byte layout with one big-endian auth ID and two fixed 30-byte fields.
- Implementation: `RenaissanceGameLogin<'a>` returns the big-endian auth ID and borrowed raw username/password prefixes; `decode_renaissance_game_login` validates exact length and ID, then reuses the existing NUL-prefix helper at the fixed offsets.
- Residual boundary: decoded values remain a raw wire representation. Grant consumption, credential validation, reconnect acceptance, and transport remain deferred.
- Current status: implementation and isolated checks complete; local stacked work-unit commit is the delivery boundary. Commit subject: `feat(protocol): decode Renaissance game login`.

## Next step
Continue with PRE-SST as a separate application-owned session-selection transition; keep reconnect-grant consumption and credential policy in their own dependency-ordered slices.
