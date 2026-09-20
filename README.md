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
cargo run -p rustuo-server
```

## License

The repository is licensed under GPL-2.0-only. The `legacy/` directory contains ServUO code and its original license/copyright notices. Consult `legacy/LICENSE` and the project history before redistributing derived work.
