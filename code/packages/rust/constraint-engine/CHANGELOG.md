# Changelog — `constraint-engine`

## 0.3.0 — 2026-11-02

VER10: a QF_BV tactic.

### Added

- `bv::BvTactic` decides conjunctions over `Bool` and `BitVec(w)`
  variables (`1 ≤ w ≤ 512`) by bit-blasting them with the new
  `bit-blaster` crate and solving with `cdcl-sat`. Supported: `#b`/`#x`/
  `(_ bvN w)` literals; `bvnot bvneg bvand bvor bvxor bvadd bvsub bvmul`
  (n-ary, left-associative); `bvshl bvlshr bvashr`; unsigned and signed
  comparisons; `=`, `distinct`, Boolean connectives and `ite`. Anything
  else answers `Unknown` with a reason.
- **Certified UNSAT:** the tactic logs a DRAT proof and runs `drat-check`
  on it before reporting `Unsat`. A rejected proof is `Unknown`.
- Bounds: widths ≤ `MAX_BV_WIDTH` (512), circuits ≤ `MAX_AIG_NODES`
  (4 million, checked per operation), `BV_CONFLICT_BUDGET` (1 000 000).
- `Value::BitVec(Vec<bool>)` (least significant bit first), with
  `Value::as_bv_u128` and `#b…` display.

### Changed

- `Engine::check_sat` routes problems with any `BitVec` variable, or with
  logic `QF_BV`, to the BV tactic. `QF_BV` and `BitVec(1..=512)` are now
  supported; `BitVec` mixed with `Int` answers `Unknown`.
- The trivial model of an assertion-free problem gives `BitVec` variables
  all-zero values. Widths above the limit get none, so a huge unused
  declaration allocates nothing.

## 0.2.0 — 2026-11-02

VER09: the SAT tactic now runs on the `cdcl-sat` CDCL solver.

### Changed

- **`SatTactic` is an adapter over `cdcl-sat`.** The recursive DPLL over
  string-named literals is replaced by a conflict-driven clause-learning
  solver. The signature, the three outcomes, and the "variables that occur
  in no clause are `false`" model rule are unchanged. The DPLL's
  1 000 000-node budget becomes a 1 000 000-conflict budget, and running
  out still answers `Unknown`.
- **The Cargo description is now true.** It said "SAT CDCL" while the code
  was DPLL; the tactic is now CDCL, via `cdcl-sat`.
- `src/sat.rs` became `src/sat/mod.rs`.

### Security

- **NNF size guard.** `SatTactic::solve` now refuses (`Unknown`) any
  predicate whose negation normal form would exceed `sat::MAX_NNF_NODES`
  (100 000) nodes, computed in linear time without building it. `Iff`
  copies both operands, so a chain of nested `Iff`s made `to_nnf` take
  time and memory exponential in the depth (seconds at depth 20, hours at
  depth 30, far below the depth guard) before any budget applied. That
  case existed before VER09; the security review of this change found it.

### Added

- `sat::CONFLICT_BUDGET`, `sat::MAX_NNF_NODES`.
- `src/sat/dpll_oracle.rs`: the pre-VER09 DPLL, unchanged, compiled only
  under `cfg(test)`. A new test runs 1000 random propositional formulas
  through the adapter, the old DPLL and a truth table, and requires all
  three to agree; every model is checked with `eval_bool`.

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
