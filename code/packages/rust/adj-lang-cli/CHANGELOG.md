# Changelog

## [0.1.0] - 2026-06-10 — initial CLI driver

### Added

- `adj-lang-cli PROGRAM.adj` — the CPU-bound reasoner entry point for the adj-lang
  DSL. Reads a `.adj` program (rulebook clauses + `observe`/`?` lines), compiles it
  via `adj_lang::compile`, runs `adj_lang::decide` (the `logic_engine` differential),
  and emits JSON: the ranked hypotheses with per-step proof DAGs, and the decision
  (`determinate` / `kickback` / `empty`). **Zero model calls.**
- Each proof step is joined back to its firing clause and emits the cited
  `source` / `locator` / `trust` tier — the byte-cited audit trail, reconstructable
  without the model. Non-finite numbers serialize as JSON `null`.
- Declarative argument parsing via `cli-builder` (embedded JSON spec). Exit codes:
  0 ok, 1 compile error (`{"error": ...}`), 2 bad args / unreadable file.
- 3 golden tests (`tests/cli_golden.rs`): single-hypothesis cited proof DAG,
  two-hypothesis differential ranking + decision, compile-error-as-JSON.
