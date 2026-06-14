# Changelog — twig-to-wasm

All notable changes to this crate are documented here.

## [0.1.1] — 2026-05-13

### Fixed

- **`br_table` handler** — the WASM execution engine's `br_table` dispatch
  table was misread when the target index exceeded the table size (should
  use the default label, was indexing past the end).  `fib(10)` now returns
  `55` through the WebAssembly runtime.
- End-to-end test `fib_returns_55` added to `tests/test_pipeline.rs`.

## [0.1.0] — 2026-05-11

### Added

- **`compile_twig_to_wasm(source, module_name) → Result<Vec<u8>, TwigToWasmError>`**
  — the single public entry point.  Compiles a Twig source string to a WASM
  1.0 binary in one call.

- **`TwigToWasmError`** — error enum with three variants:
  - `CompileError(TwigCompileError)` — Twig parse or name-resolution error.
  - `WasmError(IIRWasmError)` — IIR → WASM validation or lowering error.
  - `EncodeError(String)` — WASM binary encoding failure.
  All implement `std::error::Error` with full source chaining and `Display`.

- **`pre_lower_builtins` pass** — unconditional pipeline-local pass that
  converts `call_builtin "+"` → `add`, `call_builtin "="` → `eq`,
  `call_builtin "_move"` → `mov`, etc. *before* type inference.  The WASM
  backend uses the short comparison op names (`"eq"`, `"lt"`, `"gt"`, `"le"`,
  `"ge"`) rather than the `"cmp_*"` form used by the BEAM backend.

- **`fixup_control_flow_types` pass** — pipeline-local pass that repairs
  `"any"` type hints on control-flow instructions (`ret`, `call`, `jmp_if_*`,
  `label`) and on arithmetic ops whose operands are function parameters.
  Seeds the SSA env with parameters as `"i64"`.  Handles `mov` with
  passthrough typing.  Uses `"eq"` / `"lt"` / etc. names for WASM comparison
  fixup (matching the WASM lowering convention).

- **30 integration tests** in `tests/test_pipeline.rs`:
  - Group 1: successful compilations (addition, subtraction, multiplication,
    division, equality, comparisons, nested arithmetic, factorial, Fibonacci,
    multiple functions, `if`-expressions, deep nesting, mutual recursion,
    non-trivial binary size).
  - Group 2: compile errors (syntax errors, unbound names, lambda with unbound
    capture, unbalanced parentheses).
  - Group 3: WASM backend errors (empty program → `make_nil`, nil literal).
    Boolean literal test is intentionally non-asserting (either outcome valid).
  - Group 4: error type properties (Display non-empty, std::error::Error,
    source chain non-nil).
  - Group 5: binary structure (magic `\x00asm`, version `[1,0,0,0]`,
    determinism, different programs → different binaries, function name
    embedded in WASM export section).

- **4 doc-tests** in `src/lib.rs`, `src/pipeline.rs`, and `src/error.rs`.

### Implementation notes

- The WASM backend uses short comparison op names: `"eq"`, `"lt"`, `"gt"`,
  `"le"`, `"ge"` — not the `"cmp_*"` prefix used by the BEAM backend.
  The BUILTIN_MAP and fixup pass both use these short names.

- `_move` maps to `"mov"` (the WASM backend supports a `mov` op that emits
  `local.get` + `local.set`), unlike the BEAM pipeline which uses `load_reg`.

- WASM 1.0 does NOT embed the module name string in the binary.  The `module_name`
  argument is passed to `IIRWasmConfig` but `lower_iir_to_wasm` currently
  ignores it (`_config` parameter).  Function names appear in the export
  section; module name does not.  Test 5.5 checks function name embedding
  (not module name).
