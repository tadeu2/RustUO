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
- Remote operations were outside the original authorization. The user later explicitly authorized delivery of this completed slice to `tadeu2/RustUO` as issue #9 and PR #10, merged to `main` only after required checks passed; this does not authorize future unrelated remote work.
- Technical artifacts and code remain in English. Use Conventional Commits and do not add co-author attribution.
- Strict TDD is enabled (source: existing `openspec/config.yaml`); runner: `cargo test -p rustuo-server`. For behavior changes, observe RED before implementation, then GREEN and REFACTOR. For a test-only proof of behavior that already exists, run the test first and record an immediate pass honestly; do not manufacture a failing state.
- Required validation: `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace`, and `git diff --check`.

## Authorized Scope
- Source: `crates/rustuo-protocol/src/lib.rs`, `crates/rustuo-server/src/lib.rs`, and new `crates/rustuo-server/src/reconnect_admission.rs` only.
- Tracking: this document and Engram mirror `odd/legacy-account-reconnect-integration/tasks`.
- Local commits are authorized for completed work units on this feature branch.
- Remote delivery was explicitly authorized by the user on 2026-09-25, limited to issue #9 and PR #10 on `tadeu2/RustUO` `main`; PR #10 has been merged after green checks.

## Acceptance Criteria
- The reconnect admission API builds independently from the feature branch's committed tree; it does not depend on dirty source in the original checkout.
- A valid legacy XML account can authenticate through `RepositoryCredentialVerifier` and reconnect admission, returning the stored legacy username identity and the grant's exact version value.
- Unknown grant IDs short-circuit before verification; failed credentials consume the grant and a replay is rejected.
- No account policy, password migration, listener, packet dispatch, or startup wiring is added.
- Strict TDD was applied honestly: behavior changes use RED/GREEN/REFACTOR; test-only proof of already-existing behavior is run first and may pass immediately. All required validation commands have observed results.
- The original checkout's status and contents remain unchanged.

## Route and Forecast
- Route: delegated direct, one writer for read-for-write preparation and implementation, followed by parent structural readback and spot-check.
- Trigger evidence: the work touches multiple non-trivial Rust modules; source preparation spans protocol, application admission, and repository composition.
- Observed authored change: 400 added lines across the two source commits and this tracker; this is the approximate planning heuristic, not a delivery gate. Generated `.codegraph/` data is excluded and must not be committed.
- Delivery: one issue-linked PR to `main`, merged with a merge commit. The source diff was 329 additions, below the approximate 400-line planning budget; no size exception or PR chain was needed.
- Effective TDD: strict, from `openspec/config.yaml`; test runner `cargo test -p rustuo-server`.

## Checklist
- [x] LAR-1 — Extract the minimal transport-neutral reconnect grant/admission boundary into a standalone server module, retaining the already-decoded login value contract; add focused behavior tests. Commit: `41cb20b` (`feat(server): isolate reconnect admission boundary`).
- [x] LAR-2 — Prove end-to-end composition from the XML repository through its verifier adapter into reconnect admission, including success and consumed-grant failure behavior. Commit: `1270511` (`test(server): cover XML reconnect admission composition`).
- [x] LAR-DEL-1 — Publish the completed reconnect-admission slice through its approved issue-linked PR. Issue #9 was approved; PR #10 merged to `main` as `d24270ab0eecbdec1663c5f5f41c5c30b4c42e81` after the PR checks passed.

## Progress
- LAR-1 committed as `41cb20b`: added the decoded `RenaissanceGameLogin` value and a generic-version reconnect grant/admission boundary, with tests for successful one-time consumption, unknown-grant short-circuiting, and failed-credential consumption/replay rejection.
- Strict TDD evidence: the focused tests first failed on the absent login/admission API, then passed after implementation. The writer reported `cargo fmt --all -- --check`, `CARGO_NET_OFFLINE=true cargo check --workspace`, `CARGO_NET_OFFLINE=true cargo test --workspace`, and `git diff --check` all passing.
- Independent verification and parent spot-check both passed `CARGO_NET_OFFLINE=true cargo test -p rustuo-server` (15 passed, 0 failed); diff is limited to the three authorized source paths. CodeGraph did not index the new module, so the verifier inspected the scoped diff directly.
- Follow-up coverage note: the 128-entry eviction, repeated-ID collision exhaustion, and issuer-error paths are implemented but not directly tested; they were outside LAR-1's stated acceptance criteria and are not claimed as test-verified.
- LAR-2 committed as `1270511`: two test-only XML repository/verifier/admission cases verify the stored account identity and exact grant version, then verify failed credentials consume the grant and reject replay. Both cases assert the XML bytes remain unchanged.
- LAR-2's test-first run passed immediately (5 passed, 0 failed) because the production composition already worked. No meaningful RED existed without inventing a broken assertion; this was recorded rather than manufacturing one.
- Writer reported formatting, offline workspace check, offline workspace tests (17 passed), and `git diff --check` passing. Independent verification confirmed the commit changes only the admission test module; parent spot-check `CARGO_NET_OFFLINE=true cargo test --workspace` also passed (17 passed, 0 failed).
- No listener, packet decoding, session handoff, or startup integration was added; runtime harness remains N/A because there is no listener in this slice.
- The original dirty checkout remains unchanged; all implementation work is isolated to the sibling feature worktree.
- Remote delivery used a clean sibling worktree based on `main` at `7a732cb`; only commits `41cb20b` and `1270511` were transplanted, preserving the three-source-file scope and excluding the stale local task branch ancestry.
- On the approved candidate, `cargo fmt --all -- --check`, `CARGO_NET_OFFLINE=true cargo check --workspace`, `CARGO_NET_OFFLINE=true cargo test --workspace` (17 server tests passed), and `git diff --check origin/main...HEAD` passed. Independent source review found no blockers; the PR and post-merge `main` CI checks also passed.
- PR #10 merged on 2026-09-25 as `d24270ab0eecbdec1663c5f5f41c5c30b4c42e81`; the approved delivery does not add listener/startup/packet-dispatch integration or authorize future unrelated remote operations.

## Next Step
The tracked repository-to-verifier-to-admission composition is now proven. A later transport slice should first map packet/session entry points and decide the account-identity handoff contract; do not infer startup or live-socket readiness from these transport-neutral tests.

## Relevant Files
- `crates/rustuo-server/src/account_repository.rs` — committed XML repository and repository-backed verifier adapter.
- `crates/rustuo-server/src/lib.rs` — committed verifier port and module composition root.
- `crates/rustuo-protocol/src/lib.rs` — existing protocol types; only the already-decoded game-login value may be added here.
- `crates/rustuo-server/src/reconnect_admission.rs` — standalone application-level reconnect-grant/admission module and XML-composition tests.
