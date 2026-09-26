# PRE-IAV: In-Memory Account Verifier

## Boundary

Add an application-owned `AccountId` and a deterministic fixture implementing the existing borrowed-byte `ReconnectCredentialVerifier` port in `crates/rustuo-server/src/lib.rs`.

`InMemoryAccountRecord` owns exact raw username and password bytes. `InMemoryAccountVerifier` compares both fields byte-for-byte, returns the matching `AccountId`, and distinguishes unknown usernames from password mismatches using credential-free typed errors. This is a test/development fixture only: it stores raw passwords, performs no hashing, and MUST NOT be used as production authentication.

The fixture is exercised through `admit_renaissance_reconnect`; admission still consumes a grant before verification, returns its stored `ClientVersion` on success, and never restores a grant after rejection.

## Excluded

Storage, persistence, hashing, account migration, IP/access/ban/lifecycle policy, production authentication policy, RNG, network, packet dispatch, and reconnect handoff. `legacy/` remains read-only.

## Local stacked delivery

- Parent: `8c2be3ed3211d223982d2e34379d88295bde1a4e`.
- Scope: this task file and `crates/rustuo-server/src/lib.rs` only; no push, PR, merge, remote operation, or root mixed-WIP edit.
- Strict TDD on an isolated clean `git archive HEAD` candidate: tests-only RED failed because fixture API imports did not exist; GREEN passed all five focused fixture tests.
- Exact-candidate validation: `cargo test -p rustuo-server`, `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace`, and `git diff --check`. Final counts and commit evidence are reported at handoff.
