# Renaissance reconnect response compression

## Legacy evidence

For a successful classic 5.0.8.3 game login (`0x91`), ServUO enables
compression before sending `SupportedFeatures` and `CharacterListOld`:
`legacy/Server/Network/PacketHandlers.cs:3017-3031`. The first frame is classic
`0xB9` with a 16-bit big-endian feature mask, followed by old `0xA9`.
`legacy/Server/Network/Packets.cs:3304-3345,4842-4903,5400-5428` documents
those packet shapes. `legacy/Server/Network/NetState.cs:642-656` compresses each
outbound packet separately. `legacy/Server/Network/Compression.cs:14-160`
provides the fixed UO Huffman table, MSB-first codes, terminal code `0xD`, and
zero padding to the next byte. This is not zlib or Deflate.

The feature mask is an explicit session input. Renaissance/UOR can produce
`0x0003`, but `legacy/Server/ExpansionInfo.cs:46-86,175-181` and
`legacy/Config/Expansion.cfg:15` show that server expansion and account slot
configuration can change it; `0x0003` is not a universal default.

## Wire checks

Reference bytes were calculated independently by reading the legacy table as
`(bit count, code)` pairs and appending each code MSB-first, then terminal
`0xD` and zero padding. They were not calculated with the Rust compressor.

- Raw `B9 00 03` -> `B3 06 9A`; raw `B9 12 34` -> `B3 39 96 1D`.
- Raw 309-byte old `A9` list for Alice -> 85-byte compressed vector asserted
  literally in the world-entry integration test.
- The seeded 37-byte `1B` LoginConfirm ->
  `48 01 F0 0F AE 97 94 6B 54 55 11 8B 16 2C 40 0B 23 88 00 1A`.

The session returns two distinct reconnect writes in order, compressed `B9`
then compressed `A9`. After valid slot zero, it returns a separately compressed
`1B`. Invalid or replayed phases and unsupported slots remain rejected.

## Boundary

This is a pure protocol and transport-neutral session slice. It does not
complete DoLogin, a socket transport, subsequent world packets, or real-client
compatibility proof. No runtime listener is launched here.

## Output limit

ServUO's compression buffer is exactly 64 KiB
(`legacy/Server/Network/Compression.cs:48-62,85-88,118-125,149-156`). It
reports no compressed packet if writing would exceed that buffer. The pure
Rust compressor returns a typed `OutputOverflow` error instead of returning
partial bytes, and the world-entry session propagates that error before a
phase transition. The 11-bit `0xA6` code makes 47,662 repetitions produce an
exactly valid 65,536-byte result; 47,663 and 65,535 repetitions exceed the
limit and are rejected. Small packet vectors and empty-input terminal behavior
remain unchanged.
