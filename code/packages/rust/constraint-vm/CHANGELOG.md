# Changelog — `constraint-vm`

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
