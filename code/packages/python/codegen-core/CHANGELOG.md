# Changelog — codegen-core

All notable changes to this package will be documented in this file.

---

## [0.1.0] — 2026-04-27

### Added — LANG19: Initial release — unified IR-to-native compilation layer

`codegen-core` is the new shared package that defines what "code generation"
means in this repository.  Every compilation path — JIT, AOT, and compiled
languages — passes through it.

**Problem it solves**

Before LANG19, the repository had two completely parallel "lower IR to native
binary" pipelines that shared no code:

- JIT/AOT: `list[CIRInstr]` → `jit_core.optimizer.run()` → `backend.compile()`
- Compiled languages: `IrProgram` → `ir-optimizer` → language-specific compiler

There was also a backwards dependency: `aot-core` depended on `jit-core`
just to get `optimizer.run()` and `CIRInstr`.

**New module: `codegen_core.cir`**

- `CIRInstr` — the typed intermediate instruction shared by the JIT and AOT
  specialisation passes.  Moved here from `jit_core.cir`; `jit_core.cir`
  now re-exports it for backwards compatibility.

**New module: `codegen_core.backend`**

- `Backend[IR]` — generic structural protocol for any backend.  Generic
  over the IR type: `Backend[list[CIRInstr]]` for the JIT/AOT path;
  `Backend[IrProgram]` for the compiled-language path.
- `BackendProtocol` — alias for `Backend`, kept for backwards compatibility
  with callers that imported it from `jit_core.backend`.
- `CIRBackend` — convenience alias for `Backend[list[CIRInstr]]`.

**New module: `codegen_core.pipeline`**

- `Optimizer[IR]` — protocol: any object with `run(ir: IR) -> IR`.
- `CodegenPipeline[IR]` — universal optimize-then-compile pipeline.
  - `compile(ir: IR) -> bytes | None` — fast path.
  - `compile_with_stats(ir: IR) -> CodegenResult[IR]` — returns timing,
    IR snapshot, and backend name alongside the binary.
  - `run(binary, args)` — pass-through to the backend.

**New module: `codegen_core.result`**

- `CodegenResult[IR]` — result of `compile_with_stats()`.  Fields:
  `binary`, `ir` (post-optimization snapshot), `backend_name`,
  `compilation_time_ns`, `optimizer_applied`, `success`, `binary_size`.

**New module: `codegen_core.registry`**

- `BackendRegistry` — name-to-backend map with `register()`, `get()`,
  `get_or_raise()`, `names()`, `__contains__`.

**New module: `codegen_core.optimizer.cir_optimizer`**

- `run(cir: list[CIRInstr]) -> list[CIRInstr]` — constant folding + DCE.
  Moved from `jit_core.optimizer`; `jit_core.optimizer` re-exports it.
- `CIROptimizer` — class-based wrapper satisfying `Optimizer[list[CIRInstr]]`
  for use in `CodegenPipeline`.

**New module: `codegen_core.optimizer.ir_program`**

- `IrProgramOptimizer` — wraps `IrOptimizer` from `ir-optimizer` into the
  `Optimizer[IrProgram]` protocol.  Exposes `run(ir: IrProgram) -> IrProgram`
  for the fast path and `optimize_with_stats(ir)` for diagnostics.

**New tests: `tests/`** — 71 tests across four files:

- `test_cir.py` (13) — `CIRInstr` construction, `__str__`, predicates.
- `test_cir_pipeline.py` (27) — `CodegenPipeline[CIR]` fast path, stats,
  optimizer integration.
- `test_ir_pipeline.py` (10) — `CodegenPipeline[IrProgram]` + `IrProgramOptimizer`.
- `test_registry.py` (12) — `BackendRegistry` all operations.
- `test_optimizer.py` (22) — constant folding (all foldable ops), DCE, combined
  `run()`, `CIROptimizer` class.

Coverage: 91.35%.
