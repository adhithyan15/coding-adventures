# Changelog — `constraint-engine`

## Unreleased — 2026-08-18

Brought under CI: the crate had no `BUILD` file, so nothing ever ran its tests
or linted it.

### Added

- **`BUILD` file — this crate is now built, tested and linted in CI.**

  This crate is a member of the `code/packages/rust` workspace, so it compiled
  whenever a sibling with a `BUILD` file pulled it in as a path dependency. But
  the build tool discovers work by scanning for `BUILD` files, so with none of
  its own it was never a package in its own right: its **test targets were never
  compiled, its assertions never ran, and `cargo clippy --all-targets -- -D
  warnings` never linted it**, on any platform. Adding `BUILD` puts it under the
  same per-package clippy gate and test run as every other watched Rust crate.

  The BUILD is the repo-standard one-liner, `cargo test -p constraint-engine -- --nocapture`,
  kept on a single line: the build tool runs each BUILD line as its own
  `sh -c`, so a backslash continuation would silently truncate the command.
  It was verified green locally first — clippy `-D warnings` clean and a full
  unfiltered `cargo test --no-fail-fast` passing — per the "expect to find
  existing breakage when you start watching a long-unwatched package" rule in
  `lessons.md`.

## 0.1.1 — 2026-05-04

Security hardening — DoS guards on the SAT and LIA tactics.

### Fixed

- **SAT tactic: depth check before CNF conversion** (`sat.rs`).  The SAT
  tactic now calls `depth_of(p) > MAX_PREDICATE_DEPTH` on every assertion
  before invoking `to_cnf()`.  Predicates exceeding the depth limit return
  `SolverResult::Unknown(…)` immediately rather than recursing into stack
  overflow.  The `to_cnf()` call is also guarded via `Result` — a
  `Err(budget_msg)` returns `Unknown(budget_msg)` so the engine degrades
  gracefully to "can't decide" instead of panicking.

- **LIA tactic: `neqs` changed from `Vec<i128>` to `HashSet<i128>`**
  (`lia.rs`).  The candidate-filter step inside `eliminate_all` called
  `neqs.contains(&candidate)` in a tight loop.  With `Vec`, O(n) membership
  cost per candidate turned into O(n²) when a user supplied hundreds of
  disequality constraints (e.g. `x ≠ 1, x ≠ 2, …, x ≠ k`).  `HashSet`
  makes each `.contains()` O(1).

## 0.1.0 — 2026-05-04

Initial release.  **LANG24 PR 24-C.**

### Added

- `Engine` struct: declare variables, assert predicates, `check_sat`, `snapshot`/`reset_all` for scope management.
- `SolverResult` enum: `Sat(Model)`, `Unsat`, `Unknown(String)`.
- `Model` struct: variable → `Value` mapping with `get`/`insert`/`iter`.
- `Value` enum: `Bool(bool)`, `Int(i128)`, `Real(i128, i128)`.
- **LIA tactic** (`lia` module): bounded Cooper variable-elimination for `QF_LIA`.
  - Handles `Ge`, `Le`, `Lt`, `Gt`, `Eq`, `NEq`, `Add`, `Sub`, `Mul` predicates.
  - Multi-variable constraints solved via sequential elimination.
  - Fixed: deferred evaluation of constraints over unbound variables to prevent spurious UNSAT.
- **SAT tactic** (`sat` module): DPLL with unit propagation and pure-literal elimination for `QF_Bool`.
- Nelson-Oppen-style dispatch: integer vars → LIA, bool-only → SAT, mixed → LIA.
- Trivial model generation for empty assertion sets.
- `eval_bool` / `eval_int_or_bool` for model verification.
- 46 unit tests covering both tactics and engine dispatch.
