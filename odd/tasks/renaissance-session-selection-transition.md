# ODD Task: Renaissance Session Selection Transition

## Objective

Add the first application-owned session transition for Renaissance server selection, independent of sockets and transport handoff.

## Scope

- Keep an authenticated session snapshot of offered endpoints (address and port).
- Validate the signed `RenaissanceServerSelection` index against that snapshot.
- Obtain an auth ID through an injected `AuthIdIssuer`.
- On success, advance the session to `AwaitingGameReconnect` and return the selected endpoint, auth ID, and protocol-owned `0x8C` acknowledgement bytes.
- Add focused unit tests for exact acknowledgement bytes, invalid indexes, issuer failure, and repeated selection.

## Constraints

- Preserve all existing mixed tracked and untracked WIP; do not reset, stash, normalize, or remove unrelated changes.
- Keep `legacy/` read-only.
- Keep protocol codecs in `rustuo-protocol`; do not duplicate wire serialization.
- Do not add `ClientVersion`, reconnect-grant storage, account authentication, token persistence/validation, sockets, listeners, packet dispatch, reconnect acceptance, or live connection handoff. `ClientVersion` and grant lifecycle are the dependent PRE-RGW task.
- Treat the `0x8C` response as a client reconnect target; do not infer a live server-side socket transfer.
- Keep generated technical artifacts in English.
- Strict TDD: RED -> GREEN -> REFACTOR.
- Local conventional work-unit commit on `codex/legacy-account-bridge`, following the user-selected `stacked-to-main` strategy. No push, PR, remote, merge, deploy, or credential operation.

## Authorized scope

- Source: `crates/rustuo-server/src/lib.rs`.
- Task evidence: this file and its Engram mirror.
- Validation: focused and required repository checks; do not modify unrelated source/config/OpenSpec paths.

## Checklist

- [x] SST-1 — Add focused RED tests for session selection, rollback, and phase transitions.
- [x] SST-2 — Implement endpoint validation, injected auth issuance, phase transition, and prepared `0x8C` result.
- [x] SST-3 — Run isolated focused and repository checks and record evidence.

## Acceptance criteria

- A valid signed index selects only an endpoint offered by the authenticated session.
- Negative and out-of-range indexes return typed errors without issuing an ID or mutating session state.
- An issuer failure leaves the session unchanged.
- Successful selection returns the endpoint, auth ID, and exact `0x8C` bytes, then transitions to `AwaitingGameReconnect`.
- A repeated selection is rejected without issuing another ID.
- The slice does not add grant storage, authentication, transport, reconnect acceptance, or handoff.

## Verification evidence

- Base commit: `cc38d649676569e591449953b65c8a9693eab9af` (`PRE-GLD`).
- TDD RED: on a clean archive of the base commit with only four focused tests added, `cargo test -p rustuo-server session_selection` failed with exit 101 and `E0432` for the missing session API, as expected. Snapshot: `/tmp/rustuo-pre-sst-red.zPGDev`.
- TDD GREEN: `cargo test -p rustuo-server session_selection` — 4 passed, 12 filtered.
- `cargo fmt --all -- --check` — passed.
- `cargo check --workspace` — passed.
- `cargo test --workspace` — passed: 39 unit tests (core 1, protocol 22, server 16, world 0); all doc-tests passed.
- All verification ran on a clean isolated candidate based on the base commit, containing only this slice; unrelated mixed-worktree results are not candidate evidence.
- Runtime harness: N/A — this application transition has no socket or client runtime boundary.
- Rollback boundary: revert the local `feat(server): add Renaissance session selection transition` work-unit commit.

## Progress

- CodeGraph confirmed the committed `0xA0` selection type and `0x8C` encoder are the only protocol dependencies needed.
- A read-only SOL-medium boundary assessment confirmed SST can stand alone if it omits the later `ClientVersion` and `ReconnectGrantWindow` integration.
- The isolated candidate keeps `RenaissanceLoginSession` limited to offered endpoints and phase; `select_server` receives only the selection and injected issuer. The dirty root server file remains untouched, so grant-window, reconnect-admission, verifier, and composition WIP stays preserved.
- Local delivery is one conventional stacked work-unit commit with this task record and the session-selection source; no remote action is included.

## Next step

Continue with PRE-RGW as a separate dependency-ordered slice. It adds `ClientVersion`, bounded one-time grant storage, and selection integration without widening this session-selection commit to transport or credential policy.
