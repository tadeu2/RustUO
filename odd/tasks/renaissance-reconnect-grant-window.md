# ODD Task: Renaissance Reconnect Grant Window

## Objective
Add a transport-independent, bounded, single-use reconnect-grant window to the application-owned Renaissance session service.

## Problem
The session transition issues an auth ID for the `0x8C` response but does not retain the ID, associate it with the authenticated client version, prevent collisions, or consume it once when a later reconnect presents it.

## Why
Legacy retains a bounded auth window and restores the client version on game login. Modeling that lifecycle now keeps token ownership in the application layer without prematurely implementing packet `0x91`, sockets, or reconnect policy.

## Scope
- Reuse `rustuo_core::ClientVersion` as the stored version value.
- Add an in-memory grant window under `crates/rustuo-server/src/` with capacity 128 and oldest-first eviction.
- Issue IDs through the existing injected `AuthIdIssuer`, retrying collisions without overwriting a live grant and returning a typed exhaustion/error result if issuance cannot produce a free ID.
- Store `auth_id -> ClientVersion`, consume each grant exactly once, and return `None` for unknown or already-consumed IDs.
- Adapt `RenaissanceLoginSession` so successful server selection registers the issued ID for its authenticated version before committing `AwaitingGameReconnect`.
- Add focused tests for storage, one-time consumption, unknown IDs, collision handling, oldest eviction, issuer failure, and selection/ack integration.

## Constraints
- Preserve the observed 27-entry dirty/untracked WIP; do not reset, stash, normalize, or remove unrelated changes.
- Keep `legacy/` read-only.
- Do not decode or interpret packet `0x91`, validate credentials, implement account authentication, add fallback client-verification policy, persist grants, add sockets/listeners/dispatch, accept reconnects, or perform live handoff.
- Keep the application/session service independent of transport; accept an auth ID directly for later consumers.
- Keep protocol codecs in `rustuo-protocol`; do not duplicate `0x8C` serialization.
- Keep production RNG/token-source selection out of scope; the ID source remains injected.
- Technical artifacts remain in English.
- Strict TDD is enabled: RED -> GREEN -> REFACTOR.
- Test command: `cargo test -p rustuo-server`; repository checks: `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace`.
- Delivery is a local stacked conventional commit on `codex/legacy-account-bridge`; no push, PR, merge, deployment, credentials, or real client.

## Authorized scope
- Source: `crates/rustuo-server/src/lib.rs` and, only if required by the existing API, the server crate manifest or `main.rs`.
- Task tracking: `odd/tasks/renaissance-reconnect-grant-window.md` only.
- Verification may read the repository and run the commands above; no other source/config/OpenSpec paths may be modified.

## Checklist
- [x] RGW-1 — Add grant-window and session-version contracts with focused RED tests.
- [x] RGW-2 — Implement collision-safe bounded issuance, one-time consumption, eviction, and selection integration.
- [x] RGW-3 — Refactor only if needed, run focused and repository checks, and record evidence.

## Acceptance criteria
- A successful selection registers the issued auth ID with the authenticated `ClientVersion` before the session transition commits.
- A grant is consumed once; unknown and repeated IDs return `None` without creating or restoring state.
- Live IDs are never overwritten by collisions; the bounded window evicts only its oldest entry at capacity 128.
- Issuer failures and collision exhaustion leave session phase and live grants unchanged.
- No packet `0x91`, account auth, fallback verification policy, persistence, sockets, dispatch, reconnect acceptance, or live handoff is added.
- Focused and required repository checks report observed results.

## Route
- RGW-1/RGW-2/RGW-3: delegated direct implementation by one writer, followed by parent spot-check and independent verification because the current native assessment is high/unassessable while unrelated untracked WIP exists.
- Trigger evidence: the slice extends an application service across session state, bounded lifecycle storage, collision semantics, and multiple tests; preparation requires multiple source and legacy references.

## Progress
- Exploration complete: CodeGraph and a read-only mapper found the existing `AuthIdIssuer` only creates an ID; no Rust grant lifecycle exists.
- Legacy evidence: auth entries retain creation/version data, cap the process-wide window at 128, evict oldest entries, avoid live-ID collisions, and remove an ID on game login; unknown-token fallback and connection checks are separate policy.
- Architecture decision: keep grants application-owned and transport-neutral; keep ID generation injected and defer production RNG choice.
- Implemented in `crates/rustuo-server/src/lib.rs`: `ReconnectGrantWindow` stores `(auth_id, ClientVersion)` in a `VecDeque`, bounds capacity at 128, retries collisions up to 128 issuance attempts, evicts the oldest grant only when a distinct free ID is ready, and consumes known IDs exactly once. `RenaissanceLoginSession` now retains its authenticated `ClientVersion`; successful selection registers the grant before transitioning to `AwaitingGameReconnect`, while grant errors leave the phase/window unchanged. The protocol crate remains the only `0x8C` encoder.
- TDD evidence: RED `cargo test -p rustuo-server reconnect_grant_window` failed as expected on missing `ReconnectGrantWindow` / `ReconnectGrantError` and the new constructor/selection API. GREEN passed seven focused grant tests and four preserved session-selection tests covering one-time/unknown consumption, collision retry without overwrite, full-window oldest eviction, issuer failure, collision exhaustion, selection/ack version integration, and session/window rollback, plus prior selection cases.
- REFACTOR: no behavioral refactor was useful; applied `rustfmt --edition 2021` to the authorized server library source and removed two unused test-loop bindings. Final focused tests pass without compiler warnings.
- Isolated candidate verification: `cargo test -p rustuo-server reconnect_grant_window` passed (7 tests); `cargo fmt --all -- --check`, `cargo check --workspace`, and `cargo test --workspace` passed (1 core, 22 protocol, 23 server, 0 world tests). The final two-path staged patch passed `git diff --cached --check`. No required check remains pending.
- WIP preservation evidence: the candidate was created from `git archive a01b78d0`; only the authorized server source and this task record enter the local commit. Root mixed WIP remains unstaged and unmodified.
- Residual risk/scope boundary: collision retries are bounded at 128; a continuously colliding/non-diverse injected issuer returns `CollisionExhausted`. Production ID-source selection, packet `0x91`, credentials, persistence, reconnect acceptance, and transport remain intentionally deferred.
- Local delivery: this two-path PRE-RGW slice is stacked directly on `a01b78d0dc7ce581bb49ca1db783fa28f52f6d7f`; no remote operation is included.

## Next step
Reconnect-grant window complete. Keep packet `0x91`, credential validation, and live reconnect/transport handling for later authorized slices.
