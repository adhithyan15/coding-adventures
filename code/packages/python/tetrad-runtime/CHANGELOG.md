# Changelog

All notable changes to `tetrad-runtime` will be documented in this file.

## [Unreleased]

### Added — LANG18: Source-line coverage via DebugSidecar composition

LANG18 layers source-level coverage reporting on top of the IIR-level coverage
collected by `vm-core` (LANG18 vm-core).  The key insight is that the
`DebugSidecar` built by `sidecar_builder` already maps every IIR instruction
index back to a `(file, line, col)` — we simply compose that with the
set of executed IIR indices.

**New module: `tetrad_runtime.coverage`**

- `CoveredLine` — dataclass: `file: str`, `line: int`, `iir_hit_count: int`.
  `iir_hit_count` is the number of *distinct* IIR instruction indices at that
  source line that were executed (not how many times the line ran — for frequency
  use LANG17 `BranchStats`).

- `LineCoverageReport` — report dataclass containing `covered_lines: list[CoveredLine]`
  with two helpers:
  - `lines_for_file(path) -> list[int]` — sorted covered line numbers for one file.
  - `total_lines_covered() -> int` — total number of distinct `(file, line)` pairs.
  - `files() -> list[str]` — sorted list of unique source files in the report.

- `build_report(iir_coverage, sidecar_bytes) -> LineCoverageReport` — projection
  function.  For every `(fn_name, ip_set)` entry in `iir_coverage` (from
  `VMCore.coverage_data()`), calls `DebugSidecarReader.lookup(fn_name, ip)` to
  obtain a `SourceLocation`, accumulates the `(file, line)` pairs, and builds the
  report.  IIR instructions with no sidecar entry (synthetic preamble instructions)
  are silently skipped.

**New method: `TetradRuntime.run_with_coverage(source, source_path) -> LineCoverageReport`**

  End-to-end entry point:
  1. Calls `compile_with_debug(source, source_path)` to get `(module, sidecar)`.
  2. Creates a fresh `VMCore` with `enable_coverage()`.
  3. Executes the module.
  4. Reads `vm.coverage_data()` and calls `build_report(iir_cov, sidecar)`.
  5. Returns the `LineCoverageReport`.

**Updated exports** in `tetrad_runtime/__init__.py`:
- `CoveredLine` and `LineCoverageReport` are now exported from the package root.
- `__all__` updated accordingly.

**New tests: `tests/test_coverage.py`** — 19 tests in five classes:
  `TestReturnType`, `TestBasicCoverage`, `TestTwoFunctionCoverage`,
  `TestBuildReport`, `TestTotalLinesCovered`.  Full suite passes at 95.89%
  coverage.

### Added — LANG06: source-map composition sidecar and debug execution API

The LANG06 debug integration connects every stage of the Tetrad pipeline
into a single end-to-end debugger story: set a breakpoint by source line →
VM pauses → inspect frame state → resume.

**New module: `tetrad_runtime.sidecar_builder`**

- `code_object_to_iir_with_sidecar(main, source_path, module_name="tetrad-program") -> (IIRModule, bytes)`
  — performs the standard Tetrad → IIR translation and additionally builds a
  `DebugSidecar` that maps every IIR instruction index in every function back
  to the original Tetrad source file, line, and column.

  The core composition algorithm:
  ```
  CodeObject.source_map:   (tetrad_ip=7)  → (line=3, col=5)
  IIRFunction.source_map:  (iir_start=14) → (tetrad_ip=7)
  ──────────────────────────────────────────────────────────────
  Composed:                (iir_start=14) → (line=3, col=5)
  ```
  Written into a `DebugSidecar` via `DebugSidecarWriter`.
  The `DebugSidecarReader` then answers `lookup(fn, ip)` → `SourceLocation`
  and `find_instr(file, line)` → IIR index for breakpoint resolution.

  Variable declarations are also written:
  - Parameters: one `Variable` per `code.params[i]`, `reg_index=i`, live
    for the full function body.
  - Locals: one `Variable` per `code.var_names[len(params):]`, live from 0
    (conservative but correct for the Variables panel).

**New `TetradRuntime` methods**

- `compile_with_debug(source, source_path) -> (IIRModule, bytes)` — compile
  source and build the sidecar in one call.  Stores the sidecar in
  `self._last_sidecar`.
- `run_with_debug(source, source_path, *, hooks=None, breakpoints=None) -> Any`
  — compile, attach hooks, pre-set breakpoints (as `{fn_name: [iir_idx, ...]}`),
  then execute.  The `DebugHooks` subclass receives `on_instruction` for every
  pause, `on_call` on every function entry, `on_return` on every function exit,
  and `on_exception` on unhandled errors.  `hooks=None` → zero debug overhead.

**Updated `tetrad_runtime/__init__.py`**

- Re-exports `code_object_to_iir_with_sidecar` from the package root.
- Docstring updated to document all four new debug-related entry points.

**Updated dependencies**

- `pyproject.toml` — added `coding-adventures-debug-sidecar` as a runtime
  dependency (required by `sidecar_builder`).
- `BUILD` — added `-e ../debug-sidecar` to the `uv pip install` chain.

**New test module: `tests/test_debug_integration.py`** (29 tests across 4 classes)

- `TestSidecarBuilder` — verifies `compile_with_debug` returns valid sidecar
  bytes, registers the source file and both function names, stores the sidecar
  on the runtime, and produces the same structure as the standalone function.
- `TestSourceLineQueries` — verifies `find_instr(file, line)` resolves known
  source lines to non-negative IIR indices; `lookup(fn, iir_idx)` returns the
  expected `SourceLocation`; nearest-preceding lookup semantics hold; both
  functions in a two-function program are reachable.
- `TestLiveVariables` — verifies parameters and locals appear in
  `live_variables(fn, 0)` with `type_hint="u8"`; `main` has no variables.
- `TestRunWithDebug` — verifies: correct return values; `on_instruction` fires
  at the breakpoint IIR index; `reader.lookup(fn, ip)` inside the hook resolves
  to the expected source line; `step_in` visits multiple instructions and
  enters callees; `call_stack()` shows the callee function on top when paused
  inside it; multiple breakpoints in the same function all fire.

### Changed — Intel 4004 codegen extracted into intel4004-backend

The `Intel4004Backend` class and the codegen / IR it depends on
moved out of `tetrad-runtime` and into a new sibling package,
`intel4004-backend`.  This establishes the per-target
`<arch>-backend` package pattern future native backends will follow
(`intel8008-backend`, `mos6502-backend`, `riscv32-backend`, …).

- `tetrad_runtime._intel4004_codegen` subpackage removed.  Its
  `codegen.py` and `ir.py` now live as `intel4004_backend.codegen`
  / `intel4004_backend.ir`.
- `tetrad_runtime.intel4004_backend` becomes a thin shim that
  re-exports `intel4004_backend.Intel4004Backend`.  Callers that
  reach `tetrad_runtime.Intel4004Backend` continue to work; new
  code should import from `intel4004_backend` directly.
- `pyproject.toml` swaps the direct `intel4004-simulator` dep for
  `intel4004-backend` (which depends on the simulator transitively).
- `BUILD` adds `-e ../intel4004-backend` to the `uv pip install`
  chain.

The 33 codegen tests + 5 backend-adapter tests moved with the code
into `intel4004-backend/tests/`.  `tetrad-runtime` keeps its 67
runtime / translator / parity tests; coverage jumped to 94.7%
because the inlined codegen (which had ~92% coverage) is no longer
counted toward `tetrad-runtime`'s lines.

### Changed — inline the Intel 4004 codegen, drop legacy package deps

The bytecode → 4004 codegen and the SSA-by-name `IRInstr` it consumes
moved in-tree under `tetrad_runtime._intel4004_codegen` (subpackage,
underscore-prefixed to mark it internal).  Originally these lived in
the now-retired `tetrad-jit` package; the only consumer was
`Intel4004Backend`, so co-locating them with their sole user
simplifies the dependency graph and unblocks deletion of `tetrad-jit`.

- `tetrad_runtime._intel4004_codegen.ir` — `IRInstr`, `evaluate_op`
- `tetrad_runtime._intel4004_codegen.codegen` — `codegen`,
  `run_on_4004`, `DeoptimizerError`
- `Intel4004Backend.compile` / `.run` now import from the in-tree
  module instead of `tetrad_jit`; the deprecated import-fallback
  branch was removed.
- The 13 `evaluate_op` tests and 20 `codegen` tests from
  `tetrad-jit/tests/test_tetrad_jit.py` (the `TestEvaluateOp` and
  `TestCodegen` classes) moved here as `tests/test_intel4004_codegen.py`.

### Removed

- `coding-adventures-tetrad-vm` and `coding-adventures-tetrad-jit`
  removed from `pyproject.toml` `dependencies` and from `BUILD`.
  `intel4004-simulator` remains (still needed for `run_on_4004`).

### Added — LANG17 PR4: legacy ``TetradVM`` API parity

Adds re-projection wrappers on `TetradRuntime` so callers can switch
from the legacy `tetrad_vm.TetradVM` to `tetrad_runtime.TetradRuntime`
without rewriting metric-reading code.

- `TetradRuntime.hot_functions(threshold=100)` — delegates to
  `VMCore.hot_functions`.
- `TetradRuntime.feedback_vector(fn)` → `list[SlotState] | None`,
  indexed by Tetrad slot index.  Reconstructed by walking
  `IIRFunction.feedback_slots` (populated in the translator).  Returns
  `None` for fully-typed functions (which allocate no slots).
- `TetradRuntime.type_profile(fn, slot)` → `SlotState | None` —
  one-slot lookup over `feedback_vector`.
- `TetradRuntime.call_site_shape(fn, slot)` → `SlotKind` — returns
  `UNINITIALIZED` for unknown / unreached slots, matching legacy.
- `TetradRuntime.branch_profile(fn, tetrad_ip)` → `BranchStats | None`,
  re-keyed from the IIR's IIR-IP-keyed counters via
  `IIRFunction.source_map`.
- `TetradRuntime.loop_iterations(fn)` → `dict[tetrad_ip, int]`,
  re-keyed via `source_map`.
- `TetradRuntime.execute_traced(source)` → `(result, list[VMTrace])`,
  thin wrapper over `VMCore.execute_traced`.
- `TetradRuntime.reset_metrics()` — delegates to live `VMCore`.

The translator (`code_object_to_iir`) now populates
`IIRFunction.feedback_slots` and `IIRFunction.source_map` so the
re-projection layer has the data it needs:

- `feedback_slots[slot_index]` → IIR index of the value-producing
  instruction that gets the observation (i.e. the last IIR
  instruction in that Tetrad op's translation).
- `source_map` → list of `(iir_start, tetrad_ip, 0)` per Tetrad
  instruction, so any Tetrad IP resolves to the IIR range of its
  translation.

Test coverage: 68 tests, 94% line coverage.  New file
`test_legacy_api_parity.py` covers shape parity for every legacy
metric API plus the empty-state-defaults contract.

## [0.1.0] — 2026-04-23

### Added

- Initial release — Tetrad reimplemented on top of the LANG pipeline.
- `compile_to_iir(source)` translator: Tetrad source → standard-opcode `IIRModule`.
- `code_object_to_iir(code, name)` translator: existing Tetrad `CodeObject`
  → `IIRModule`, allowing pre-compiled programs to be re-targeted to the
  LANG pipeline without reparsing.
- `TetradRuntime` façade: wraps `vm_core.VMCore` with the Tetrad-specific
  builtins (`__io_in`, `__io_out`, `__get_global`, `__set_global`) and a
  small set of Tetrad opcode extensions (`tetrad.move`).
- `TetradRuntime.run(source)` — end-to-end interpreted execution.
- `TetradRuntime.run_with_jit(source)` — JIT path through `jit_core.JITCore`
  with an `Intel4004Backend`.  Functions the 4004 backend cannot compile
  fall back to interpretation transparently.
- `Intel4004Backend` — adapts `intel4004-simulator` to the LANG
  `BackendProtocol`.
- Test coverage: end-to-end runs of the canonical Tetrad demo programs
  (arithmetic, control flow, function calls, globals, I/O).

### Notes

- This package lives **alongside** the legacy `tetrad-vm` and `tetrad-jit`
  packages rather than replacing them.  Both paths share the same
  `tetrad-compiler` front end.  Future work will retire the legacy
  packages once `tetrad-runtime` reaches parity on every metric and
  diagnostic the legacy packages expose.
- The Intel 4004 backend currently reuses `tetrad-jit`'s
  `codegen_4004.py` by translating CIR → tetrad-jit's IR shape.
  A native CIR-aware codegen will follow once the backend protocol
  conventions stabilise.
