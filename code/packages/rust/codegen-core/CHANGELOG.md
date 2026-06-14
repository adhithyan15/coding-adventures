# Changelog — codegen-core (Rust)

## [0.1.0] — 2026-04-28

### Added

Initial Rust port of the Python `codegen-core` package (LANG19 / LANG20).

#### `optimizer` module
- `Optimizer<IR>` trait — generic IR → IR transformation protocol
  (`fn optimize(&self, ir: IR) -> IR`).
- `impl Optimizer<Vec<CIRInstr>> for CIROptimizer` — blanket adapter so
  `jit_core`'s constant-folding + DCE optimizer can be used in a
  `CodegenPipeline<Vec<CIRInstr>>`.
- `IrProgramOptimizer` — thin wrapper around `ir_optimizer::IrOptimizer`
  that implements `Optimizer<IrProgram>` (strips `IrOp::Nop` instructions).

#### `codegen` module (LANG20 protocol)
- `CodeGenerator<IR, Assembly>` trait — validate + generate assembly
  (`fn name()`, `fn validate(&IR)`, `fn generate(&IR)`).
- `CodeGeneratorRegistry` — name-to-generator map backed by
  `HashMap<String, Box<dyn Any + Send + Sync>>` with `register`, `get`,
  `names`, `len`, `is_empty`, and `Default`.

#### `pipeline` module
- `Compile<IR>` trait — generic compilation backend (`name + compile`).
- Blanket `impl<B: Backend> Compile<Vec<CIRInstr>> for B` so any
  `jit_core::backend::Backend` works with `CodegenPipeline<Vec<CIRInstr>>`
  without adapter boilerplate.
- `CodegenResult<IR>` — compilation output struct with `binary`,
  `ir_snapshot`, `backend_name`, `compilation_time_ns`, `optimizer_applied`,
  plus `success()` and `binary_size()` derived helpers.
- `CodegenPipeline<IR>` — composes `Option<Box<dyn Optimizer<IR>>>` +
  `Box<dyn Compile<IR>>`; exposes `compile(ir)` (fast path) and
  `compile_with_stats(ir)` (diagnostics path with timing + IR snapshot).

#### `registry` module
- `BackendRegistry` — name-to-backend map backed by
  `HashMap<String, Box<dyn Any + Send + Sync>>` with `register`, `get`,
  `get_or_raise`, `names`, `len`, `is_empty`, and `Default`.

#### Crate root re-exports
- From `jit-core`: `Backend`, `CIRInstr`, `CIROperand`, `CIROptimizer`.
- From own modules: `CodeGenerator`, `CodeGeneratorRegistry`,
  `IrProgramOptimizer`, `Compile`, `CodegenPipeline`, `CodegenResult`,
  `Optimizer`, `BackendRegistry`.

### Tests
- 41 unit tests across all four modules.
- 17 doc tests embedded in rustdoc examples.
- 58 total tests; all green.
