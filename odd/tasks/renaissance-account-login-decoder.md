# ODD Task: Renaissance Account Login Decoder

## Objective
Add a transport-independent protocol decoder for the Renaissance `0x80` account-login frame.

## Problem
The protocol crate needs a typed boundary for the first client login packet. This pure decoder exposes the observed wire fields without coupling to framing, authentication, session state, or dispatch.

## Why
Decoding one concrete packet keeps the migration vertical and source-grounded: it captures the legacy fixed-width fields and stops before transport or server behavior.

## Scope
- Add a pure `0x80` account-login decoder in `crates/rustuo-protocol/src/lib.rs`.
- Validate the fixed frame shape observed in legacy: packet ID `0x80` and 62 bytes total.
- Read username and password from their fixed 30-byte fields; terminate each exposed value at its first NUL while consuming the full field width.
- Preserve the raw non-NUL bytes; do not assume UTF-8, authenticate, normalize, log, or otherwise interpret credentials.
- Accept the final frame byte without assigning it protocol meaning, because the legacy handler leaves it unread.
- Add focused tests for valid decoding, NUL termination and fixed-width consumption, truncation, wrong ID/length, and arbitrary trailing-byte acceptance.

## Constraints
- Preserve all unrelated mixed-worktree WIP; do not normalize or remove unrelated changes.
- Keep `legacy/` read-only.
- Do not add authentication, server-list responses, handler dispatch, session state, socket integration, or `0xEF` handling.
- Do not assume text encoding or add validation not evidenced by the legacy wire format.
- Technical artifacts remain in English.
- Strict TDD is enabled: RED -> GREEN -> REFACTOR.
- Test command: `cargo test -p rustuo-protocol`; repository checks: `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace`.
- Local work-unit commit on `codex/legacy-account-bridge` is authorized under `stacked-to-main`; no push, PR, merge, remote, or credential operations.

## Authorized scope
- Source: `crates/rustuo-protocol/src/lib.rs`
- Task tracking: `odd/tasks/renaissance-account-login-decoder.md` and its Engram mirror.
- Verification may read the repository and run the commands above; no other source/config/OpenSpec paths may be modified.

## Checklist
- [x] ALD-1 — Add the account-login API and focused RED tests.
- [x] ALD-2 — Implement fixed-width raw-byte decoding with explicit packet/length errors.
- [x] ALD-3 — Refactor only if needed, run focused and repository checks, and record evidence.

## Acceptance criteria
- A valid 62-byte `0x80` frame returns the username and password bytes without UTF-8 assumptions.
- NUL-terminated fields expose only bytes before the first NUL while consuming all 30 bytes of each field.
- Frames with the wrong ID, wrong length, or truncation return explicit errors without panicking.
- Any value in the final byte is accepted without assigning it semantic meaning.
- No authentication, dispatch, session, socket, or `0xEF` behavior is added.
- Focused and required repository checks report observed results.

## Route
- ALD-1/ALD-2/ALD-3: delegated direct implementation was completed in mixed WIP; this work unit selectively extracts the decoder and its five focused tests onto PRE-RSD HEAD for isolated proof and local commit.
- Trigger evidence: the slice adds non-trivial parsing semantics and tests to the protocol module while preserving a broad dirty worktree.

## Progress
- Exploration complete: CodeGraph and legacy references identified `0x80` as a pure protocol boundary independent of the uncommitted `PacketInputBuffer`.
- Legacy evidence: `legacy/Server/Network/PacketHandlers.cs` registers `0x80` at length 62 and reads `ReadString(30)` twice; `legacy/Server/Network/PacketReader.cs` stops returned strings at NUL but advances to the full fixed-width field end.
- Implementation complete: `decode_renaissance_account_login` returns borrowed raw-byte username/password fields, validates packet ID and exact 62-byte length, reports truncation separately, and accepts the final byte without interpretation.
- Historical mixed-worktree TDD evidence: focused RED failed as expected because `AccountLoginDecodeError` and `decode_renaissance_account_login` were not yet defined; after implementation `cargo test -p rustuo-protocol` passed (34 tests). The extraction did not alter behavior.
- Historical mixed-worktree evidence only: earlier writer and independent checks passed 34 focused tests and 62 workspace unit tests; those counts do not prove this isolated candidate.
- Isolated source candidate on PRE-RSD `5d693635868e2124a61ed1767dba5fed62aa6a1c` (source tree `6db66c7bbb4150c03825b4c0f23e3848adfd0c05`) passed `cargo test -p rustuo-protocol account_login_decoder` (5 passed, 4 filtered), `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace` (22 unit tests, 0 failed), and `git diff --check`.
- Residual risks: field bytes are intentionally not decoded as text and are not authenticated or normalized; the legacy login handler's authentication/session behavior remains out of scope.
- Current status: selective decoder extraction and isolated source checks complete. The local commit identity will be recorded in the Renaissance MVP tracker after commit; parent spot-check remains pending.

## Next step
After local commit, continue with PRE-ALA only; keep authentication, dispatch, and session behavior out of this task.
