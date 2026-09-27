# Renaissance client world fixture

## Scope

- Add a separate, in-memory Trammel fixture for client-facing tests.
- Keep `World::new()` as the synthetic map `0xFE`, 16 × 16, player `(8, 8, 0)` fixture.
- Do not add map data, terrain, world-entry packets, or client runtime behavior.

## Legacy evidence

- `legacy/Scripts/Misc/MapDefinitions.cs` registers Trammel as map `1`, sized `7168 × 4096`.
- `legacy/Scripts/Accounting/AccountHandler.cs` places the default New Haven Bank start at `(3503, 2574, 14)`.
- `legacy/Server/Network/Packets.cs` makes a `CityInfo` without an explicit map default to `Map.Trammel`. The separate T2A list explicitly uses Felucca and is not this fixture's source.

## TDD evidence

- RED: Test-only `rustuo-world` change failed with `E0599`: `World::renaissance_client_fixture` was absent.
- GREEN: The focused Renaissance fixture and existing synthetic fixture tests passed after adding the constructor and instance-local map metadata.

## Validation boundary

The fixture proves in-memory map identity, half-open bounds, seeded player, and one eastward move. It does not prove actual client login, world entry, map-file loading, or terrain behavior.
