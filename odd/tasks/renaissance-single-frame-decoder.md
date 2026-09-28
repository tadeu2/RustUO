# Generic single-frame decoder

## Legacy evidence

`legacy/Server/Network/MessagePump.cs:233-267` looks up a packet handler by ID, takes its fixed `Length` or reads a variable length from the three-byte header, rejects variable lengths below three, and waits for the full packet before consuming bytes. `legacy/Server/Network/PacketHandler.cs:5-29` stores the ID and length. Legacy code remains read-only.

## Scope

Add a transport-independent, borrowed single-frame decoder to `crates/rustuo-protocol/src/lib.rs`. A caller registers fixed or variable packet layouts, then receives one complete frame and its untouched remainder, `None` for incomplete input, or a typed layout/ID/length error. This unit does not register Renaissance packet IDs and does not implement multi-frame iteration, an input buffer, sockets, authentication, or sessions.

## Verification

Tests cover fixed and variable framing, borrowing/remainders, incomplete headers/bodies, unknown IDs, invalid variable lengths, zero fixed lengths, and duplicate registrations. Run `cargo test -p rustuo-protocol`, `cargo fmt --all -- --check`, `cargo check --workspace`, `cargo test --workspace`, and `git diff --check` in the isolated worktree.
