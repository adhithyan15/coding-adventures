# Changelog — twig-to-cil

All notable changes to this crate are documented here.

## [0.1.0] — 2026-05-11

### Added

- Initial release.
- `compile_twig_to_cil(source, module_name) -> Result<CILProgramArtifact, TwigToCilError>` —
  the primary public API.
- `pipeline::run_pipeline(source, module_name, config)` — pipeline entry point
  with a custom `IIRClrConfig` for callers who need a non-default assembly name.
- `error::TwigToCilError` — unified error enum with variants for each pipeline
  stage: `Compile`, `TypeCheck`, `ClrValidation`, `ClrBackend`.
- Four-stage pipeline:
  1. `twig-ir-compiler` — Twig source → `IIRModule`.
  2. `iir-type-checker` — type inference + validation.
  3. `iir-builtin-lowering` — `call_builtin "+"` → `add` rewriting.
  4. `iir-to-cil-bytecode` — IIR → `CILProgramArtifact`.
- 25 integration tests in `tests/test_pipeline.rs` covering:
  - Basic arithmetic (`+`, `-`, `*`, `/`) with opcode-level assertions
  - Comparisons (`=`, `<`, `>`)
  - Conditionals (`if`, nested `if`, `if` with comparison condition)
  - Named functions (square, factorial, fibonacci)
  - Multiple functions and mutual recursion
  - Artifact structure invariants (non-empty bodies, `ret` presence, entry method)
  - `let` binding and `begin` sequence
  - Two-argument functions
  - Error cases (broken syntax, unbound variable, unbound lambda capture)
