# Changelog — `constraint-engine`

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
