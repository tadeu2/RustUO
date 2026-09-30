# RustUO

RustUO is an experimental Rust reimplementation of an Ultima Online server.

## Repository layout

- `crates/rustuo-core/` — shared server primitives and future runtime contracts.
- `crates/rustuo-protocol/` — client/server protocol types and packet handling.
- `crates/rustuo-world/` — world, entities, persistence, and simulation boundaries.
- `crates/rustuo-server/` — executable server entry point.
- `legacy/` — original ServUO C# source retained for reference and incremental migration.

The Rust workspace is intentionally minimal. It is a clean foundation, not yet a replacement for ServUO.

## Development

```bash
cargo check --workspace
cargo test --workspace
cargo run -p rustuo-server -- --accounts path/to/accounts.xml
```

The executable is a **loopback-only smoke server**, not a production server. It
loads a ServUO-compatible `accounts.xml` read-only, processes clients serially,
and uses a fixed one-avatar world/login-tail fixture plus sequential local
reconnect IDs. After a successful reconnect it expects character slot zero
(`0x5D`) and serves one movement request before closing that game connection.
`--once` exits after that movement reply; otherwise it accepts another client.
The default listener is `127.0.0.1:2593`; `--listen` accepts only IPv4 loopback
addresses. Do not expose this fixture to a network. Passing the protocol tests
does not prove compatibility with an actual Renaissance client.

```bash
cargo run -p rustuo-server -- \
  --accounts path/to/accounts.xml \
  --listen 127.0.0.1:2593 \
  --once
```

## License

The repository is licensed under GPL-2.0-only. The `legacy/` directory contains ServUO code and its original license/copyright notices. Consult `legacy/LICENSE` and the project history before redistributing derived work.
