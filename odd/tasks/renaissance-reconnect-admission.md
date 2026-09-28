# ODD Task: Renaissance Reconnect Admission (PRE-RGA)

## Objective
Provide a transport-neutral admission boundary for an already-decoded Renaissance game login. A known reconnect grant is consumed before injected credential verification; the result carries the grant's stored client version and the verifier's account value.

## Scope and constraints
- Add `RenaissanceReconnectAdmission<Account>`, `ReconnectAdmissionError<E>`, and `admit_renaissance_reconnect` in `crates/rustuo-server/src/lib.rs`.
- Use the existing `ReconnectCredentialVerifier` port and `ReconnectGrantWindow`; do not add a production verifier or account rules.
- Unknown/replayed auth IDs do not invoke the verifier. A known ID remains consumed if verification returns an error.
- Pass borrowed raw username/password slices through unchanged; never copy, log, or persist them in this service.
- No packet decoding changes, client/seed checks, session attachment, transport, dispatch, connection acceptance, or live handoff. `legacy/` remains read-only.
- Preserve all unrelated root WIP. Local stacked delivery only; no push, PR, merge, remote, deploy, credentials, or real client.

## Work unit
- [x] RED: On a clean `git archive` of base `1b0fa41178c8a64dc5d93f459e2d5b7d49ad7f35`, four admission tests failed compilation with missing admission function and result/error types (E0432).
- [x] GREEN: Implemented only the admission boundary and its typed outcomes. Focused `cargo test -p rustuo-server reconnect_admission_tests`: 4 passed, 0 failed.
- [x] REFACTOR: Applied Rust formatting to the isolated candidate; no behavior refactor was needed.
- [x] Verified the final isolated candidate: `cargo test -p rustuo-server` passed (27 server tests); `cargo fmt --all -- --check`, `cargo check --workspace`, and `cargo test --workspace` passed (1 core, 22 protocol, 27 server, 0 world tests; doc tests passed). Candidate diff check passed.

## Acceptance evidence
- Accepted grant returns the stored `ClientVersion` and generic account value. A test compares the verifier's received slice pointers and lengths to the input slices, including non-UTF-8 bytes.
- Unknown and replayed IDs return `UnknownAuthId` with no additional verifier call.
- Verifier error returns `VerifierFailed(E)` and the grant cannot be consumed again.
- Runtime harness: N/A; this boundary has no socket, live client, or transport integration.
- Rollback boundary: only `crates/rustuo-server/src/lib.rs` and this task record. Existing SST/RGW behavior and tests remain in place.

## Delivery
One local Conventional Commit on the existing stacked branch `codex/legacy-account-bridge`, parent `1b0fa41178c8a64dc5d93f459e2d5b7d49ad7f35`, containing exactly the two paths above. This task is a single reviewable PRE-RGA slice, not a PR or live reconnect claim.

## Next step
Keep production credential policy, session/connection acceptance, and transport integration as separate work units.
