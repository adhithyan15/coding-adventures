# Changelog

All notable changes to `tetrad-runtime` will be documented in this file.

## [Unreleased]

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
