# Changelog

## [0.1.0] — 2026-04-12

### Added

- `StepTrace` frozen dataclass: captures `pc_before`, `pc_after`, `mnemonic`, `description` for a single instruction execution.
- `ExecutionResult[StateT]` frozen generic dataclass: carries `halted`, `steps`, `final_state`, `error`, and `traces` from a full program run.
- `ExecutionResult.ok` property: `True` only when `halted=True` and `error=None`.
- `Simulator[StateT]` Protocol: structural interface with `load`, `step`, `execute`, `get_state`, and `reset` methods.
- `__init__.py` exports: `Simulator`, `ExecutionResult`, `StepTrace`.
- `py.typed` marker for mypy support.
- 22 unit tests covering immutability, generics, structural subtyping, and the end-to-end test loop.
- Full Knuth-style literate README with analogies, diagrams, and usage examples.
