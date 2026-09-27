# ODD Task: Renaissance Old Character Entry Codecs

## Objective
Add pure legacy `0xA9` character-list serialization and fixed `0x5D` slot extraction for the single pre-seeded avatar path.

## Contract
- Encode one raw name of at most 30 bytes in slot zero, padded with NUL bytes; emit five 60-byte slot records, zero cities, and `0x00000014` (`SlotLimit | OneCharacterSlot`) flags.
- Emit the old `0xA9` variable-frame header with a 309-byte big-endian total length.
- Decode exactly 73 bytes beginning with `0x5D`, returning only the signed big-endian `i32` at offsets 65..69.
- Reject overlong names, wrong packet IDs, truncated frames, and extra bytes. Preserve raw names and signed slot indexes without interpreting or authorizing them.

## Legacy evidence
- `legacy/Server/Network/Packets.cs:4842-4903` writes the old five-slot minimum character list, 60 bytes per slot, city count, and flags.
- `legacy/Server/ExpansionInfo.cs:90-98` defines the two selected flag bits.
- `legacy/Server/Network/PacketHandlers.cs:2404-2460` reads the character slot from the fixed PlayCharacter request.

## Boundaries
This task does not change framing layouts, authenticate or select a character, bind a world entity, add character creation, open sockets, or claim actual-client acceptance. `legacy/` remains read-only. Runtime integration is a later work unit.

## TDD and verification
- RED: `cargo test -p rustuo-protocol renaissance_old_character_entry` failed with `E0432` because both codec APIs and their errors were missing.
- GREEN: the focused tests cover exact list bytes, the 30-byte boundary, signed slot preservation, wrong ID, truncation, and extra bytes.
- Required final checks: `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace`, and `git diff --check`.

## Status
- [x] Add failing codec tests.
- [x] Implement the narrow pure codecs.
- [x] Record final validation and local commit.
