# Renaissance 5.0.8.3 MVP tasks

## Outcome

Deliver one end-to-end path in which a Renaissance 5.0.8.3 client authenticates against legacy XML account storage, selects a server and reconnects, enters one in-memory world as a pre-seeded avatar, and moves once. This MVP does not include normal character selection; verify that the actual client accepts the pre-seeded spawn without it.

## Existing building blocks

- `rustuo-server` has a storage-independent account repository contract, a legacy XML repository, and a credential-verifier adapter.
- Protocol code has Renaissance account-login decoding, packet framing, and related packet/session components.
- Server code has server-selection/reconnect session logic and reconnect credential verification components.
- `rustuo-world` creates an in-memory fixture player and exposes movement decision/apply behavior.
- These are separate components, not yet one running TCP-to-world flow. The server entry point still prints a bootstrap message.

## Checklist

- [ ] **R5-MVP-01 — Trace the legacy path.** Follow the relevant ServUO account login, server selection/reconnect, world-entry, and movement behavior. Record the compatibility contract and add Rust tests or compatibility checks before changing behavior. Keep `legacy/` read-only.
- [ ] **R5-MVP-02 — Compose authentication and reconnect.** Connect Renaissance packet handling to legacy XML account authentication, server selection, and game reconnect. Add focused tests for the composed path and preserve existing protocol/session boundaries.
- [ ] **R5-MVP-03 — Enter the seeded world.** Compose the authenticated session with the one in-memory pre-seeded avatar and implement the client-visible world-entry sequence. Add tests that prove the client reaches that avatar without adding normal character selection.
- [ ] **R5-MVP-04 — Complete one movement end to end.** Connect a client movement request to world decision/apply and the corresponding response. Add a focused compatibility check, then smoke-test with the actual Renaissance 5.0.8.3 client. Confirm or revise the pre-seeded-spawn assumption based on client behavior.
- [ ] **R5-MVP-05 — Validate and document the slice.** Run all required workspace checks and record their results:
  - `cargo fmt --all -- --check`
  - `cargo check --workspace`
  - `cargo test --workspace`

## Working constraints

- Preserve and isolate existing dirty work; do not overwrite unrelated changes or mix in speculative refactors.
- Trace legacy behavior first, test the Rust compatibility contract, and leave `legacy/` unchanged.
- Keep task completion unchecked until its outcome and applicable checks are observed.
