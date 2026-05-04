# Changelog

## [0.2.0] — 2026-04-29

### Added

- **LANG20 `JvmCodeGenerator`** — new `codegen` module implementing
  `CodeGenerator<IrProgram, JvmClassArtifact>` from `codegen-core`.
  - `name()` → `"jvm"`
  - `class_name()` — returns the configured JVM class name
  - `validate(ir)` — dry-run compile; returns errors as `Vec<String>`
  - `generate(ir)` → `JvmClassArtifact` (panics on invalid IR)
  - Default class name: `"Main"`; customise with `JvmCodeGenerator::new("MyClass")`
  - 8 unit tests + 1 doc-test

### Changed

- Added `codegen-core` to `[dependencies]` to enable the `CodeGenerator` trait implementation.

## [0.1.0] — Unreleased

- add the first Rust `ir-to-jvm-class-file` backend
- lower the current Brainfuck and Nib IR subset into verifier-friendly JVM bytecode
- emit helper methods for register access, byte/word memory, and syscalls
- validate class names and write generated classes into classpath layout safely
- add end-to-end tests for generic lowering plus Brainfuck and Nib source lanes
