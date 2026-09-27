# ODD Task: Renaissance World-Entry Session

## Objective
Compose a verified XML reconnect with the seeded client world, then emit the old character list and the first LoginConfirm packet for slot zero.

## Contract
- A session owns its `RenaissanceReconnectAdmission<LegacyAccountIdentity>`, one `World::renaissance_client_fixture()`, and explicit avatar presentation bytes/body/direction.
- Emit the existing 309-byte `0xA9` list once. Decode one exact 73-byte `0x5D` request, accept only slot zero, and map it to the seeded player.
- Emit the existing 37-byte `0x1B` encoder output using that player's serial and position and the owned world's map dimensions. No hard-coded serial in server composition.
- Wrong phase, malformed frame, and unsupported slot do not advance session state. Replayed list and play requests fail.

## Legacy evidence
- `legacy/Server/Network/Packets.cs` `CharacterListOld` writes five minimum slots and list flags; `LoginConfirm` writes mobile serial, body, position, direction, and map dimensions.
- `legacy/Server/Network/PacketHandlers.cs` `PlayCharacter` reads the slot and checks account ownership. This narrow fixture models only the admitted account's single seeded avatar, not general character persistence.

## TDD and validation
- RED: focused integration test failed with `E0432`, missing `renaissance_world_entry_session`.
- GREEN: three focused tests pass for XML login/reconnect composition, exact list/confirmation bytes, invalid slot/frame recovery, and replay rejection.
- Required final checks: focused test, `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace`, `git diff --check`.

## Boundary
This is only the first client-visible world packet. It does not implement remaining login packets, runtime transport, client handshake, in-world lifecycle, or real-client acceptance. `legacy/` is read-only.
