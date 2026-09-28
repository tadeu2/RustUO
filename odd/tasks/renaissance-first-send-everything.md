# ODD Task: Renaissance First SendEverything Pass

## Objective
Emit the next contiguous world-entry packet for the seeded Renaissance fixture after the initial self-mobile prefix.

## Contract
- `first_send_everything_packets` is valid only after `initial_self_mobile_packets` and is one-shot.
- The fixture has one visible player and no items, so its first range enumeration emits one additional old `MobileIncoming` packet for that same player.
- Reuse the initial self-mobile encoder and legacy Huffman compression; do not advance the phase on errors.

## Legacy evidence
- `legacy/Server/Network/PacketHandlers.cs:2530-2564` orders LoginConfirm, map setup, initial mobile packets, `SendEverything`, then light checks.
- `legacy/Server/Mobile.cs:7629-7680` enumerates nearby items and mobiles and sends `MobileIncoming` for visible mobiles.
- `legacy/Server/Map.cs:2369-2437` enumerates sector mobiles within bounds without excluding the querying mobile; `legacy/Server/Mobile.cs:9223-9232` permits self-visibility.
- `legacy/Scripts/Mobiles/PlayerMobile.cs:1127-1155` sends global and personal light packets on forced checks. These are next, not part of this slice.

## TDD and validation
- RED: focused integration test failed with `E0599` because `first_send_everything_packets` did not exist.
- GREEN: focused world-entry integration tests passed 8/8, including exact equality with the already vector-tested initial `MobileIncoming` and replay/phase rejection.

## Boundary
Fixture-only first pass; no terrain, item/equipment visibility, additional mobiles, clock/light computation, later login packets, TCP runtime, normal character selection, or real-client acceptance. `legacy/` stays read-only.
