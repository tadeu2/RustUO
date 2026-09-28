# ODD Task: Renaissance Account Login Ack Encoder

## Objective
Add a pure encoder for the Renaissance `0xA8` account-login acknowledgement/server-list packet.

## Problem
The protocol crate now decodes the client `0x80` login frame, but it cannot represent the deterministic server-list response that legacy emits after an accepted login. The response should remain a codec boundary, not become authentication, account lookup, or network orchestration.

## Why
The legacy `AccountLoginAck` packet is fully determined by a prepared ordered list of server entries. Encoding that shape next preserves a vertical protocol migration while keeping credential handling, event selection, session state, and sockets out of scope.

## Scope
- Add a pure `0xA8` encoder in `crates/rustuo-protocol/src/lib.rs` from an ordered list of prepared entries.
- Emit the variable header as packet ID `0xA8`, total length `u16` big-endian, unknown byte `0x5D`, and entry count `u16` big-endian.
- Encode each entry in slice order with a zero-based `u16` index, a 32-byte fixed raw name (truncate or NUL-pad), full-percent `u8`, timezone `i8`, and address `u32` big-endian.
- Keep names as caller-provided wire bytes; do not assume UTF-8 or add hidden text conversion.
- Reject a count or computed total length that cannot be represented by the variable `u16` length; never truncate silently.
- Keep the address field as an explicit `u32` wire value; do not parse IP strings or infer a different byte order in this slice.
- Add focused tests for empty, one, and multiple entries; length/count fields; indexes; name truncation/padding; signed timezone; address bytes; unknown byte; and overflow rejection.

## Constraints
- Preserve all unrelated mixed-worktree WIP; do not normalize or remove unrelated changes.
- Keep `legacy/` read-only.
- Do not add authentication, account lookup, event selection, packet sending, handler dispatch, session state, socket integration, or `0xA0` processing.
- Do not infer IPv4 semantics from the `u32`; serialize the supplied wire value exactly.
- Technical artifacts remain in English.
- Strict TDD is enabled: RED -> GREEN -> REFACTOR.
- Test command: `cargo test -p rustuo-protocol`; repository checks: `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace`.
- Local work-unit commit on `codex/legacy-account-bridge` is authorized under `stacked-to-main`; no push, PR, merge, remote, or credential operations.

## Authorized scope
- Source: `crates/rustuo-protocol/src/lib.rs`
- Task tracking: `odd/tasks/renaissance-account-login-ack-encoder.md` and its Engram mirror.
- Verification may read the repository and run the commands above; no other source/config/OpenSpec paths may be modified.

## Checklist
- [x] ALA-1 — Add the entry/API shape and focused RED tests.
- [x] ALA-2 — Implement deterministic variable-frame encoding and overflow errors.
- [x] ALA-3 — Refactor only if needed, run focused and repository checks, and record evidence.

## Acceptance criteria
- Empty entries produce a six-byte `0xA8` frame with correct length, `0x5D`, and zero count.
- Entries are encoded in order with zero-based big-endian indexes and 40-byte records.
- Names are truncated to 32 bytes or NUL-padded without text decoding; percent, signed timezone, and address bytes are preserved.
- The total length and count are encoded big-endian and overflow is returned as an explicit error.
- No authentication, account lookup, event selection, sending, dispatch, session, socket, or `0xA0` behavior is added.
- Focused and required repository checks report observed results.

## Route
- ALA-1/ALA-2/ALA-3: delegated direct implementation was completed in mixed WIP; this work unit selectively extracts the encoder, five tests, and only the seven `PacketWriter` methods needed by `0xA8` and the later `0x8C` encoder onto PRE-ALD HEAD.
- Trigger evidence: the slice adds a non-trivial variable-frame codec and boundary/overflow tests to the protocol module while preserving a broad dirty worktree.

## Progress
- Exploration complete: CodeGraph and legacy references identified `0xA8` as the deterministic response immediately following accepted `0x80` login.
- Legacy evidence: `AccountLoginAck` writes the unknown `0x5D`, count, zero-based entry indexes, 32-byte ASCII-fixed names, percent, signed timezone, and address value; the packet length is `6 + 40 * count`.
- Implementation complete: `encode_renaissance_account_login_ack` accepts an ordered slice of borrowed-name `RenaissanceServerListEntry` values and returns the deterministic variable frame. It uses `PacketWriter` for big-endian integers, truncates/pads raw names to 32 bytes, and rejects count or total-length overflow before writing.
- Historical mixed-worktree TDD evidence: focused RED failed because the encoder API and entry/error types were undefined. Initial implementation exposed a dynamic zero-padding compile error; a fixed zero buffer resolved it.
- Isolated extraction TDD: tests-only tree `8c40403e11da79a58f11bb73b0347d6f04e4893c` on PRE-ALD `dcf3f720816de383bfb0f795ba4d2cd1a518973c` ran `cargo test -p rustuo-protocol account_login_ack_encoder` RED (exit 101, unresolved encoder/type imports); source tree `3f03f26724eb87e55665e8c82980d29d2121e61c` ran the same filter GREEN (5 passed, 9 filtered, exit 0). A subsequent check-only formatter found an import-layout difference; the candidate was normalized before the final-tree freeze. The normalized source tree is `67351c3539f1acb896b6a0e872678cfdacbad5ea`.
- Historical mixed-worktree evidence only: writer, independent verifier, and parent observed 39 protocol tests and 67 workspace unit tests; these counts are not proof of the isolated candidate.
- Isolated normalized-source snapshot checks passed: `cargo test -p rustuo-protocol account_login_ack_encoder` (5 passed, 9 filtered), `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace` (27 unit tests, 0 failed), and `git diff --check`, all exit 0. The final tree including this task document was checked again before commit.
- Residual risks: the `u32` address is serialized exactly as supplied; IP interpretation and server-list source selection remain outside this codec. Raw name bytes are not text-decoded.
- Current status: selective extraction reached GREEN. Local commit remains pending; independent verification and parent spot-check of that commit will be recorded afterward.

## Next step
After a verified local commit, continue with PRE-SSD only; keep account/event flow and network sending out of this task.
