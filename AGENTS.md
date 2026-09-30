# RustUO agent guide

## 1. Purpose and responsibilities

RustUO incrementally migrates the behavior of the original ServUO/C# project
to Rust. The Rust implementation is the target; `legacy/` is the behavioral
reference.

Repository responsibilities:

- `Cargo.toml` — Rust 2021 workspace and shared package metadata.
- `crates/rustuo-core/` — identifiers, time, configuration, errors, and shared contracts.
- `crates/rustuo-protocol/` — packet codecs and client session state.
- `crates/rustuo-world/` — entities, maps, simulation, and persistence.
- `crates/rustuo-server/` — composition root, listeners, lifecycle, and administration.
- `legacy/` — original ServUO source and licensing notices.

Keep responsibilities within these crate boundaries. See [README.md](README.md)
for setup and [ARCHITECTURE.md](ARCHITECTURE.md) for the architecture.

Preserve `GPL-2.0-only` licensing and all copyright and license notices.
Keep code, identifiers, comments, and documentation in English unless the user
explicitly requests otherwise. Match the user's language in chat.

## 2. Protect existing work

- Inspect the current branch, working-tree changes, and linked worktrees before
  editing. Read the relevant source and task documents; do not treat old status
  reports as current evidence.
- Preserve tracked and untracked work. If another change overlaps the files you
  need, inspect it and resolve ownership before editing.
- Do not reset, stash, discard, or overwrite unrelated changes. Do not commit,
  push, merge, or delete branches or worktrees without explicit user approval.
- Limit each change to its purpose. Do not mix migration work with unrelated
  cleanup or mass renaming.
- Treat `legacy/` as read-only unless explicitly authorized. Even with approval,
  do not rewrite or remove legacy source without an equivalent Rust test or
  compatibility check.

## 3. Choose the workflow for the task

Use the applicable installed skills and project workflow instructions. These
rules do not replace their approval gates or backend checks.

- **Small documentation changes:** edit and check the affected document and its
  references. Do not create a separate specification just to correct wording.
- **Outcome and task tracking (ODD):** when continuing an existing tracked task,
  read its intended result and acceptance criteria. Update only the authorized
  tracker; do not create a competing checklist or mark unverified work complete.
- **Test-driven development (TDD):** for features and bug fixes, write a regression
  or behavior test first. Run it and confirm that it fails for the expected
  reason. Implement the smallest change, rerun the test, then improve structure
  only while tests remain green. Running existing tests alone is not TDD.
- **Specification-driven development (SDD):** for substantial changes involving
  new behavior, multiple components, or architectural decisions, define scope,
  expected behavior, design, and tasks through the applicable SDD workflow before
  implementation. Honor the selected automatic or interactive execution mode.
- **OpenSpec and Engram:** they store workflow artifacts; neither is proof that
  a workflow ran. For SDD, use the native dispatcher to resolve the configured
  store and next phase. Do not infer readiness from a directory, silently switch
  stores, or bypass blockers when the dispatcher or a required backend fails.

For each migration slice:

1. Trace the relevant behavior in `legacy/` and the existing Rust implementation.
2. Define the observable result and preserve the legacy contract in tests.
3. Follow the TDD cycle in the responsible crate.
4. Validate compatibility before starting another slice. Do not add speculative
   packet support without evidence that the behavior is needed.

## 4. Naming and code conventions

- Use `snake_case` for Rust files, modules, functions, and variables;
  `PascalCase` for types and traits; and `SCREAMING_SNAKE_CASE` for constants.
  Preserve the existing `rustuo-*` package names.
- Choose names that describe the domain and responsibility. Prefer `account_id`
  to an ambiguous `id`, and `decode_account_login` to `process_data`.
- Use one term consistently for one concept. Preserve distinctions such as
  login versus reconnect, and decoding bytes versus authenticating credentials.
- Prefer focused module names over generic `utils`, `helpers`, or `manager`
  names. Keep standard protocol abbreviations such as TCP and XML when clear.
- Include units when numeric values could be ambiguous, such as `timeout_ms`,
  or use a type that expresses the unit. Use predicate names such as
  `is_authenticated` or `has_account` for booleans where appropriate.
- Name tests after observable behavior, for example
  `invalid_password_rejects_login`. Follow the surrounding test style rather
  than renaming existing tests just for uniformity.
- Use descriptive `kebab-case.md` names for new prose documents, while preserving
  recognized filenames such as `AGENTS.md`, `README.md`, and `ARCHITECTURE.md`.
  Follow required workflow artifact names; do not rename existing documents
  without checking their links and consumers.
- Keep functions and modules focused on one responsibility. Use typed errors
  and explicit state transitions; do not hide failures or replace meaningful
  errors with panics for expected invalid input.
- Explain compatibility reasons and non-obvious constraints in comments. Keep
  wire packet identifiers and layouts unchanged when improving Rust names.

## 5. Verify and report

Use a separate Cargo target directory for each worktree to avoid sharing build
output with concurrent work. Set `CARGO_TARGET_DIR` to that directory, then run
all required checks before handoff:

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
```

Review the final diff for unintended changes. For documentation, also check
referenced paths, examples, and consistency with the source. Report any failing
or unavailable check explicitly; do not claim it passed or repair unrelated
work without authorization.

Fixture and loopback tests do not prove real-client compatibility. Do not claim
RustUO is playable until an actual Renaissance 5.0.8.3 client passes the required
login, world-entry, and movement smoke test. State missing client files or
environment restrictions as limitations, not successful acceptance.

Keep the final response short and in the user's language:

1. Explain what changed in plain language, in separate parts when requested.
2. State which checks ran and their results, including real-client testing when
   relevant.
3. Name the next concrete blocker or remaining step.

Distinguish local edits from commits, pushes, and merged work. Ask at most one
question at a time and wait for the answer before continuing dependent work.
Use conventional commit messages when commits are authorized. Do not add
`Co-Authored-By` trailers or AI attribution.
