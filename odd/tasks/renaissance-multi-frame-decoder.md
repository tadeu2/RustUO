# ODD Task: Multi-Frame Packet Decoder

## Objective
Extract complete borrowed packet frames from one input slice, preserving any incomplete trailing bytes.

## Scope
- Reuse `decode_packet_frame` and `PacketLayoutTable` in `rustuo-protocol`.
- Return ordered complete frames and the untouched incomplete suffix.
- Propagate existing unknown-ID and invalid-length errors without reinterpretation.
- No mutable input buffer, sockets, dispatch, authentication, session state, or Renaissance layout profile.

## Acceptance
- Empty input yields no frames and an empty suffix.
- Consecutive fixed and variable frames remain ordered and borrowed from the input.
- An incomplete trailing fixed frame or variable header/body remains untouched after a complete prefix.
- A frame error after a complete prefix is returned unchanged.

## TDD and validation
- RED: `cargo test -p rustuo-protocol` failed with E0432 because `decode_packet_frames` was absent.
- GREEN: the decoder delegates each step to `decode_packet_frame` and stops on its incomplete result.
- Focused and workspace validation: see the implementation commit and handoff receipt.

## Paths
- `crates/rustuo-protocol/src/lib.rs` — API, decoder, and focused tests.
- `odd/tasks/renaissance-multi-frame-decoder.md` — task record.
