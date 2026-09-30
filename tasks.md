# Renaissance 5.0.8.3 MVP tasks

## Outcome

Deliver one verified Renaissance 5.0.8.3 client-to-world path: authenticate against legacy XML account storage, select a server and reconnect, enter one in-memory world as a pre-seeded avatar, and move once. The final acceptance gate includes a smoke test with an actual Renaissance 5.0.8.3 client; fixture-based TCP tests do not replace that evidence.

## Current main status

`rustuo-server` now includes a serial, IPv4-loopback-only TCP smoke executable. It composes XML account login, server selection and reconnect, seeded-avatar world entry, and one movement exchange. The implementation and wire behavior are covered by Rust tests and a loopback fixture, but actual-client compatibility and acceptance of the pre-seeded spawn remain unverified.

## Existing building blocks

- `rustuo-server` has a storage-independent account repository contract, a read-only legacy XML repository, and a credential-verifier adapter.
- Protocol code has Renaissance login decoding, packet framing, session handling, legacy-compatible response encoders, and the world-entry and movement packet path.
- Server code composes login, reconnect admission, world entry, and movement in a bounded loopback runtime.
- `rustuo-world` creates an in-memory fixture player and exposes movement decision/apply behavior.

## Checklist

- [x] **R5-MVP-01 — Trace the legacy path.** Trace the relevant ServUO login, reconnect, world-entry, and movement behavior. Keep `legacy/` read-only and preserve the compatibility contract in Rust tests, including `renaissance_5083_compatibility.rs` and the server runtime/session tests.
- [x] **R5-MVP-02 — Compose authentication and reconnect.** Connect Renaissance packet handling to legacy XML credential authentication, server selection, and game reconnect. Cover the composed path with focused tests.
- [x] **R5-MVP-03 — Enter the seeded world.** Compose the authenticated session with the in-memory pre-seeded avatar and implement the client-visible world-entry sequence. Tests cover the fixture path without adding normal character selection.
- [ ] **R5-MVP-04 — Complete one movement end to end.** The loopback fixture exercises one movement request and response. Still run the exchange with an actual Renaissance 5.0.8.3 client and verify that it accepts the pre-seeded spawn; do not mark this task complete from fixture tests alone.
- [x] **R5-MVP-05 — Validate the Rust slice.** On 2026-09-30, the following required workspace checks completed successfully on the clean `main` implementation:
  - `cargo fmt --all -- --check`
  - `cargo check --workspace`
  - `cargo test --workspace`

## Working constraints

- Preserve and isolate existing dirty work; do not overwrite unrelated changes or mix in speculative refactors.
- Trace legacy behavior first, test the Rust compatibility contract, and leave `legacy/` unchanged.
- Keep task completion unchecked until its outcome and applicable checks are observed.
- Passing the loopback fixture is not proof of real-client compatibility. Do not expose the smoke executable to a network.
