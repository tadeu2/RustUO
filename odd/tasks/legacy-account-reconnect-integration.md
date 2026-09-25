# ODD Task: Legacy Account Reconnect Integration

## Objective
Prove that `LegacyXmlAccountRepository` can authenticate a Renaissance game reconnect through the application-owned reconnect-admission boundary, in a clean, locally committed work unit.

## Problem
The XML repository and its `ReconnectCredentialVerifier` adapter are committed, but the reconnect-admission implementation currently exists only in the original checkout's uncommitted WIP. There is no listener or startup composition point in the committed baseline.

## Why
The next safe step is to make the existing repository-backed verifier usable and testable at the transport-neutral admission boundary without claiming production startup wiring or importing unrelated protocol, session, core, or world WIP.

## Scope
- Isolate the minimal reconnect-grant/admission API needed to accept an already-decoded `RenaissanceGameLogin` and a generic client-version value.
- Add a focused composition test using `LegacyXmlAccountRepository`, `RepositoryCredentialVerifier`, a known reconnect grant, and `admit_renaissance_reconnect`.
- Preserve grant-first one-time consumption: unknown grants do not invoke credential verification; invalid credentials do not restore a consumed grant.
- Keep the legacy account adapter read-only and do not change password parsing, hashing, account lifecycle, access policy, or the XML file.
- Keep the API transport-neutral; do not add listener, packet dispatch, startup, or live socket handoff.

## Constraints
- Work only in the sibling worktree `RustUO-worktrees/feat-legacy-account-reconnect` on `codex/legacy-account-reconnect`, based on `e836561`.
- Preserve the original RustUO checkout exactly; do not stage or copy unrelated `.gitignore`, core entity/terrain, world, documentation, or configuration WIP.
- Only transplant the minimal existing protocol login value and reconnect-admission behavior required for a standalone build. Keep `ClientVersion` generic at the admission boundary to avoid importing unrelated core changes.
- Keep `legacy/` read-only; never write or rehash account data; never log or expose password/hash material.
- No push, fetch, pull, PR, merge, credential use, or other remote operation is authorized.
- Technical artifacts and code remain in English. Use Conventional Commits and do not add co-author attribution.
- Strict TDD is enabled (source: existing `openspec/config.yaml`); runner: `cargo test -p rustuo-server`. Observe RED before implementation, then GREEN and REFACTOR.
- Required validation: `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace`, and `git diff --check`.

## Authorized Scope
- Source: `crates/rustuo-protocol/src/lib.rs`, `crates/rustuo-server/src/lib.rs`, and new `crates/rustuo-server/src/reconnect_admission.rs` only.
- Tracking: this document and Engram mirror `odd/legacy-account-reconnect-integration/tasks`.
- Local commits are authorized for completed work units on this feature branch. Remote delivery is not authorized.

## Acceptance Criteria
- The reconnect admission API builds independently from the feature branch's committed tree; it does not depend on dirty source in the original checkout.
- A valid legacy XML account can authenticate through `RepositoryCredentialVerifier` and reconnect admission, returning the stored legacy username identity and the grant's exact version value.
- Unknown grant IDs short-circuit before verification; failed credentials consume the grant and a replay is rejected.
- No account policy, password migration, listener, packet dispatch, or startup wiring is added.
- Strict TDD RED/GREEN/REFACTOR and all required validation commands have observed results.
- The original checkout's status and contents remain unchanged.

## Route and Forecast
- Route: delegated direct, one writer for read-for-write preparation and implementation, followed by parent structural readback and spot-check.
- Trigger evidence: the work touches multiple non-trivial Rust modules; source preparation spans protocol, application admission, and repository composition.
- Estimated authored change: about 250 lines including this task tracker; under the 400-line planning heuristic. Generated `.codegraph/` data is excluded and must not be committed.
- Delivery: local-only work-unit commits; no PR strategy or remote operation is authorized by this task.
- Effective TDD: strict, from `openspec/config.yaml`; test runner `cargo test -p rustuo-server`.

## Checklist
- [x] LAR-1 — Extract the minimal transport-neutral reconnect grant/admission boundary into a standalone server module, retaining the already-decoded login value contract; add focused behavior tests. Commit: `41cb20b` (`feat(server): isolate reconnect admission boundary`).
- [ ] LAR-2 — Prove end-to-end composition from the XML repository through its verifier adapter into reconnect admission, including success and consumed-grant failure behavior. Commit: pending.

## Progress
- LAR-1 committed as `41cb20b`: added the decoded `RenaissanceGameLogin` value and a generic-version reconnect grant/admission boundary, with tests for successful one-time consumption, unknown-grant short-circuiting, and failed-credential consumption/replay rejection.
- Strict TDD evidence: the focused tests first failed on the absent login/admission API, then passed after implementation. The writer reported `cargo fmt --all -- --check`, `CARGO_NET_OFFLINE=true cargo check --workspace`, `CARGO_NET_OFFLINE=true cargo test --workspace`, and `git diff --check` all passing.
- Independent verification and parent spot-check both passed `CARGO_NET_OFFLINE=true cargo test -p rustuo-server` (15 passed, 0 failed); diff is limited to the three authorized source paths. CodeGraph did not index the new module, so the verifier inspected the scoped diff directly.
- Follow-up coverage note: the 128-entry eviction, repeated-ID collision exhaustion, and issuer-error paths are implemented but not directly tested; they were outside LAR-1's stated acceptance criteria and are not claimed as test-verified.
- The original dirty checkout remains unchanged; all implementation work is isolated to the sibling feature worktree.

## Next Step
Execute LAR-2 in this worktree: add a focused composition test proving the existing XML repository/verifier authenticates through the new admission boundary, then verify and commit that work unit. Keep adapter behavior read-only and preserve the original checkout.

## Relevant Files
- `crates/rustuo-server/src/account_repository.rs` — committed XML repository and repository-backed verifier adapter.
- `crates/rustuo-server/src/lib.rs` — committed verifier port and module composition root.
- `crates/rustuo-protocol/src/lib.rs` — existing protocol types; only the already-decoded game-login value may be added here.
- `crates/rustuo-server/src/reconnect_admission.rs` — planned standalone application-level reconnect-grant/admission module.
