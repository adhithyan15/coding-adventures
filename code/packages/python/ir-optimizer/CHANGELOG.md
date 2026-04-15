# Changelog

All notable changes to `coding-adventures-ir-optimizer` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This project uses [semantic versioning](https://semver.org/).

---

## [0.1.0] — 2026-04-12

### Added

- `IrPass` Protocol (`src/ir_optimizer/protocol.py`) — structural protocol
  requiring a `name: str` property and `run(IrProgram) -> IrProgram` method.
  Any class satisfying these two members is automatically an `IrPass` without
  explicit inheritance (PEP 544 structural subtyping).

- `OptimizationResult` dataclass (`src/ir_optimizer/optimizer.py`) — returned
  by `IrOptimizer.optimize()`. Contains the optimized `IrProgram`, the list of
  pass names that ran, and the instruction count before and after optimization.
  The `instructions_eliminated` property is `instructions_before - instructions_after`.

- `IrOptimizer` class (`src/ir_optimizer/optimizer.py`) — chains a list of
  `IrPass` instances into a sequential pipeline. Factory methods:
  - `IrOptimizer.default_passes()` — standard three-pass pipeline
    (DeadCodeEliminator → ConstantFolder → PeepholeOptimizer)
  - `IrOptimizer.no_op()` — empty pipeline, useful for baseline comparisons

- `DeadCodeEliminator` pass (`src/ir_optimizer/passes/dead_code.py`) — removes
  instructions that follow an unconditional branch (`JUMP`, `RET`, `HALT`)
  without an intervening label. Single-pass O(n) algorithm. Only unconditional
  branches make following code dead; `BRANCH_Z`/`BRANCH_NZ` are conditional and
  do not affect reachability of the fall-through path.

- `ConstantFolder` pass (`src/ir_optimizer/passes/constant_fold.py`) — merges
  `LOAD_IMM vN, k` followed by `ADD_IMM vN, vN, d` into `LOAD_IMM vN, (k+d)`,
  and `LOAD_IMM vN, k` followed by `AND_IMM vN, vN, mask` into
  `LOAD_IMM vN, (k & mask)`. Uses a `pending_load` dict to track known
  compile-time values. Clears the pending value when any non-constant
  instruction writes to the register.

- `PeepholeOptimizer` pass (`src/ir_optimizer/passes/peephole.py`) — three
  two-instruction patterns applied via fixed-point iteration (up to 10 passes):
  1. Merge consecutive `ADD_IMM vN, vN, a; ADD_IMM vN, vN, b` →
     `ADD_IMM vN, vN, (a+b)`
  2. Remove `AND_IMM vN, vN, 255` no-ops when preceded by `ADD_IMM` or
     `LOAD_IMM` on vN with value in [0, 255]
  3. Fold `LOAD_IMM vN, 0; ADD_IMM vN, vN, k` → `LOAD_IMM vN, k`

- `optimize()` convenience function (`src/ir_optimizer/__init__.py`) — calls
  `IrOptimizer.default_passes().optimize(program)` (or uses the provided pass
  list). The simplest entry point for calling code.

- Test suite (`tests/`) with 47 tests across four files:
  - `tests/test_ir_optimizer.py` — IrOptimizer, OptimizationResult, optimize()
  - `tests/test_passes/test_dead_code.py` — DeadCodeEliminator
  - `tests/test_passes/test_constant_fold.py` — ConstantFolder
  - `tests/test_passes/test_peephole.py` — PeepholeOptimizer

- `pyproject.toml` with `coding-adventures-compiler-ir` as the sole runtime
  dependency. Development dependencies: pytest, pytest-cov, ruff, mypy.
  Coverage threshold: 80%.

- `README.md` (Knuth-style literate documentation explaining IR optimization,
  the pass protocol, all three passes with before/after examples, the optimizer
  pipeline, and how to add new passes).
