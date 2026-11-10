# Changelog — `constraint-vm`

## Unreleased — 2026-11-10

Test-only follow-up to `constraint-engine` 0.5.0, which lowers the
division family, `bvsmod`, `concat`, `extract` and the extensions. No VM
code changed.

### Changed

- `tests/qfbv.rs`: the "unsupported input is refused" case used `bvudiv`,
  which is now supported; it uses `bvnand` (still outside the fragment).

### Added

- `division_family_follows_smt_lib_through_the_reader`: division by zero,
  the sign rules of `bvsmod` versus `bvsrem`, and `MIN sdiv -1`, through
  the SMT-LIB reader, the VM and the engine (UNSAT answers certified).

## Unreleased — 2026-11-02

QF_BV (VER10) now runs through the VM: `constraint-engine` 0.3.0 adds a
bit-vector tactic, and no VM code changed.

### Added

- `tests/qfbv.rs` and `tests/qfbv/*.smt2`. Each corpus file states its
  expected answers.
  - **Corpus:** 15 `.smt2` files (23 `check-sat` answers), read by
    `smt-lib-format` (a new dev-dependency) and executed by `Vm`.
  - **SAT answers:** every model is re-checked by an independent concrete
    evaluator.
  - **UNSAT answers:** the engine has already certified each one with
    `drat-check`. The test also brute-forces it when inputs total ≤ 16
    bits.
  - **Random problems:** 400 random QF_BV problems are compared against
    brute force.
  - **Refusals and limits:** cases for unsupported input, a huge unused
    declaration, Boolean connectives, reset and scoping.

## 0.1.0 — 2026-05-04

Initial release.  **LANG24 PR 24-D.**

### Added

- `Vm` struct: executes `Program` instruction streams, maintains scope stack.
- `Config`: configurable resource limits (`max_instrs`, `max_scope_depth`, `max_assertions`).
- `VmOutput`: collects `sat_results`, `models`, trace strings, instruction count.
- `VmError`: typed error enum (`NoModel`, `NoUnsatCore`, `NoPriorCheckSat`, `UnmatchedPop`, `LimitExceeded`, `EngineError`).
- Scope stack: `PushScope` snapshots the engine; `PopScope` restores it.
- `Reset` instruction: clears engine and scope stack.
- `Echo` instruction: appends to trace log.
- `SetOption` instruction: stores key-value options (currently informational).
- `check_sat(program)` convenience function: runs program, returns last SAT result.
- `get_model(program)` convenience function: runs program, returns last model.
- `ProgramBuilder`: fluent API for constructing programs without raw `ConstraintInstr`.
  - `set_logic`, `declare_int`, `declare_bool`, `assert_pred`, `assert_ge_int`,
    `assert_le_int`, `assert_eq_int`, `check_sat`, `get_model`, `push_scope`,
    `pop_scope`, `echo`, `build`.
- 24 unit tests + 1 doc-test.
