# Renaissance client MVP roadmap

Build one verified client-to-world path by composing the existing protocol, authentication, reconnect, and world components. The rough working estimate is about five coherent work units, not a schedule or delivery commitment.

## Current status on main

The server now runs a serial, IPv4-loopback-only smoke flow through XML login, server selection/reconnect, seeded-avatar world entry, ping echoes while awaiting movement, and one movement exchange. Rust compatibility and loopback tests cover that fixture path. The Renaissance 5.0.8.3 actual-client smoke is still pending, so this is not yet proof that a real client accepts the seeded spawn or completes the exchange.

## In scope

- Renaissance 5.0.8.3 login using legacy XML account storage.
- Server selection and game reconnect.
- One in-memory world and one pre-seeded avatar.
- One client movement, applied by the world and acknowledged to the client.
- Legacy-backed Rust tests or compatibility checks plus an actual-client smoke test.

## Not in scope

- Normal character selection, multiple playable characters, or persistent world state.
- Broad ServUO migration, speculative rewrites, or changes to `legacy/`.
- Treating fixture-based loopback tests as actual-client acceptance.

## Stages and exit criteria

1. **Establish the compatibility contract — complete on main.** Trace the relevant legacy login, reconnect, world-entry, and movement path. Preserve packet and state behavior in tests.
2. **Wire authentication through reconnect — complete on main.** Connect the account repository and verifier to packet handling and server selection; cover accepted and rejected credentials and reconnect admission.
3. **Compose world entry — complete on main for the fixture.** Enter the in-memory world as the pre-seeded avatar; test the association without adding normal character selection. This does not yet prove that the actual client accepts this path.
4. **Prove movement with the real client — pending.** The loopback fixture covers a movement exchange. Run a compatibility smoke with an actual Renaissance 5.0.8.3 client and explicitly verify the pre-seeded-spawn assumption. Treat incompatibility as a scope/compatibility decision, not an assumption.
5. **Close the MVP slice — checks passed, acceptance pending.** The required workspace checks passed on 2026-09-30. Close the MVP only after the actual-client path is demonstrated, or after its limitation is explicitly accepted and documented.

## After the MVP

Use evidence from the end-to-end smoke test to choose the next narrow migration slice. If the real client rejects the pre-seeded-avatar path, implement the character-entry behavior required by that evidence; otherwise expand only the next user-visible world interaction. Keep persistence and broader gameplay out until evidence justifies them.
