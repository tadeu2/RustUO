# ODD Task: Renaissance 5.0.8.3 Login Layout Profile

## Objective and boundary
Add a protocol-only layout table for the three fixed-length login packets registered by ServUO: `0x80` (62 bytes), `0xA0` (3 bytes), and `0x91` (65 bytes). Source: `legacy/Server/Network/PacketHandlers.cs` handlers at lines 98, 108, and 100 respectively.

The profile does not accept `0x5D`, `0x02`, `0x22`, `0x73`, `0xBD`, or the newer `0xEF` path. The raw four-byte Renaissance seed remains outside packet framing and in its existing decoder. No framing, buffer, server, session, authentication, world, socket, or legacy-source changes belong to this work unit.

## Evidence
- Strict TDD: tests-only RED failed because `renaissance_5083_login_layouts` was absent; two focused tests then passed GREEN.
- Fresh-target protocol suite: 43 passed, 0 failed.
- Fresh-target workspace suite: 85 passed, 0 failed; documentation tests had 0 tests.
- `cargo fmt --all -- --check`, `cargo check --workspace`, and `git diff --check` passed.
- Runtime harness: N/A; this is a pure layout-table profile, not a network or session path.

Rollback boundary: remove this helper and its tests without changing the generic frame decoder or any other crate. Delivery is one local conventional commit on `codex/legacy-account-bridge`; no remote, PR, or root-worktree operation is included.
