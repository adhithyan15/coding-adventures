# Changelog — twig-to-jvm

All notable changes to this crate are documented here.

## [0.1.0] — 2026-05-11

### Added

- Initial release.
- `compile_twig_to_jvm(source, module_name) -> Result<JvmClassFile, TwigToJvmError>` —
  the primary public API.
- `pipeline::run_pipeline(source, module_name, config)` — pipeline entry point
  with a custom `IIRJvmConfig` for callers who need a non-default class name.
- `error::TwigToJvmError` — unified error enum with variants for each pipeline
  stage: `Compile`, `TypeCheck`, `JvmValidation`, `JvmBackend`.
- Four-stage pipeline:
  1. `twig-ir-compiler` — Twig source → `IIRModule`.
  2. `iir-type-checker` — type inference + validation.
  3. `iir-builtin-lowering` — `call_builtin "+"` → `add` rewriting.
  4. `iir-to-jvm-class-file` — IIR → `JvmClassFile`.
- 22 integration tests in `tests/test_pipeline.rs` covering:
  - Basic arithmetic (`+`, `-`, `*`, `/`)
  - Comparisons (`=`, `<`, `>`)
  - Conditionals (`if`, nested `if`)
  - Named functions (`square`, factorial, fibonacci)
  - Multiple functions and mutual recursion
  - Class-file structure invariants (class name, Code attribute, constant pool)
  - Error cases (broken syntax, unbound variable, unbound lambda capture)
  - `let` binding and `begin` sequence
