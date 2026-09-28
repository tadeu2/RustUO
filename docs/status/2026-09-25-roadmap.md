# Roadmap — RustUO

Revisión documental: 2026-09-25.

## Base

RustUO es una reimplementación experimental en Rust de un servidor Ultima Online. El workspace es una base limpia y todavía no sustituye ServUO; el código legado se conserva en `legacy/`.

## Próximas fases

1. Consolidar las primitivas compartidas de `rustuo-core`.
2. Definir y estabilizar los límites de protocolo en `rustuo-protocol`.
3. Desarrollar las fronteras de mundo, persistencia y simulación en `rustuo-world`.
4. Integrar el punto de entrada de `rustuo-server`.
5. Continuar la migración incremental desde `legacy/` sin borrar la referencia original.

Esta rama contiene únicamente documentación; no incluye el WIP de implementación de la rama de desarrollo.
