# Initial architecture

The workspace is split into boundaries that can evolve independently:

1. `rustuo-core`: identifiers, time, configuration, errors, and shared contracts.
2. `rustuo-protocol`: packet codecs and client session state.
3. `rustuo-world`: entities, maps, simulation, and persistence.
4. `rustuo-server`: composition root, listeners, lifecycle, and administration.

Migration rule: port one vertical slice at a time and keep `legacy/` unchanged as the behavioural reference. Do not delete or rewrite the legacy source until an equivalent Rust test or compatibility check exists.
