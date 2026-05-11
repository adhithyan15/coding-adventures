# Changelog — twig-to-beam

All notable changes to this crate are documented here.

## [0.1.0] — 2026-05-11

### Added

- **`compile_twig_to_beam(source, module_name) → Result<Vec<u8>, TwigToBeamError>`**
  — the single public entry point.  Compiles a Twig source string to a BEAM
  binary in one call.

- **`TwigToBeamError`** — error enum with two variants:
  - `CompileError(TwigCompileError)` — Twig parse or name-resolution error.
  - `BeamError(IIRBeamError)` — IIR → BEAM validation or lowering error.
  Both implement `std::error::Error` with full source chaining and `Display`.

- **`pre_lower_builtins` pass** — unconditional pipeline-local pass that
  converts `call_builtin "+"` → `add`, `call_builtin "="` → `cmp_eq`,
  `call_builtin "_move"` → `load_reg`, etc. *before* type inference.  This is
  necessary because the type checker has inference rules for `add`/`cmp_eq`
  but not for `call_builtin "+"`.

- **`fixup_control_flow_types` pass** — pipeline-local pass that repairs
  `"any"` type hints on control-flow instructions (`ret`, `call`, `jmp_if_*`,
  `label`) and on arithmetic ops whose operands are function parameters (which
  Twig emits as `"any"`).  Seeds the SSA env with parameters as `"i64"`.
  Also handles `load_reg` (the BEAM register-copy op) with passthrough typing.

- **30 integration tests** in `tests/test_pipeline.rs`:
  - Group 1: successful compilations (addition, subtraction, multiplication,
    division, equality, comparisons, nested arithmetic, factorial, Fibonacci,
    multiple functions, `if`-expressions, deep nesting, mutual recursion,
    non-trivial binary size).
  - Group 2: compile errors (syntax errors, unbound names, lambda with unbound
    capture, unbalanced parentheses).
  - Group 3: BEAM backend errors (empty program → `make_nil`, nil literal).
    Boolean literal test is intentionally non-asserting (either outcome valid).
  - Group 4: error type properties (Display non-empty, std::error::Error,
    source chain non-nil).
  - Group 5: binary structure (FOR1 magic, BEAM tag at bytes 8..12,
    determinism, different programs → different binaries, module name
    embedded in AtU8 section).

- **4 doc-tests** in `src/lib.rs`, `src/pipeline.rs`, and `src/error.rs`.

### Implementation notes

- The pipeline deliberately does NOT use `iir_builtin_lowering::lower_builtins`
  from the sibling crate.  That function rejects `"any"` type hints (strict
  ordering guard).  The pipeline's own `pre_lower_builtins` is unconditionally
  permissive, intentionally running before inference.

- `_move` maps to `load_reg` (not `mov`) because the BEAM backend's IIR
  instruction set uses `load_reg`/`store_reg` for register copies, not `mov`.

- BEAM binary structure validated in tests: magic `b"FOR1"` at bytes 0..4,
  `b"BEAM"` at bytes 8..12, module name as atom in AtU8 section.
