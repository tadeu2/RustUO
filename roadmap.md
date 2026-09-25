# Renaissance client MVP roadmap

Build a single verified client-to-world path by composing the existing protocol, authentication, reconnect, and world components. The rough working estimate is about five coherent work units, not a schedule or delivery commitment.

## In scope

- Renaissance 5.0.8.3 login using legacy XML account storage.
- Server selection and game reconnect.
- One in-memory world and one pre-seeded avatar.
- One client movement, applied by the world and acknowledged to the client.
- Legacy-backed Rust tests or compatibility checks plus an actual-client smoke test.

## Not in scope

- Normal character selection, multiple playable characters, or persistent world state.
- Broad ServUO migration, speculative rewrites, or changes to `legacy/`.

## Stages and exit criteria

1. **Establish the compatibility contract.** Trace the legacy login, reconnect, world-entry, and movement path. Exit when the relevant packet/state behavior and test cases are written down.
2. **Wire authentication through reconnect.** Connect the existing account repository/verifier to packet handling and server selection. Exit when focused tests cover successful and rejected credentials and the reconnect handoff.
3. **Compose world entry.** Place the authenticated client into the in-memory world as the pre-seeded avatar. Exit when a composition test proves the client-to-avatar association without a normal character-selection flow.
4. **Prove movement with the real client.** Send one movement request through the server and apply it to the world. Exit when a compatibility check and an actual Renaissance 5.0.8.3 client smoke test confirm the move. Explicitly verify whether that client accepts a pre-seeded spawn without standard character selection; treat incompatibility as a scope/compatibility decision, not an assumption.
5. **Close the MVP slice.** Run `cargo fmt --all -- --check`, `cargo check --workspace`, and `cargo test --workspace`; record results and document any remaining limitation. Exit when all required checks pass and the real-client path is demonstrated, or the unresolved client limitation is clearly reported.

## After the MVP

Use evidence from the end-to-end smoke test to choose the next narrow migration slice. A natural next step is to validate and implement the character-entry behavior required by the real client, if the pre-seeded-avatar path proves incompatible; otherwise expand only the next user-visible world interaction. Keep persistence and broader gameplay out until that evidence justifies them.
