# Changelog

## [0.2.0] — 2026-04-29

### Added

- **LANG20 `Intel4004CodeGenerator`** — new `codegen` module implementing
  `CodeGenerator<IrProgram, String>` from `codegen-core`.
  - `name()` → `"intel4004"`
  - `validate(ir)` — runs the `IrToIntel4004Compiler` validator; returns errors as `Vec<String>`
  - `generate(ir)` → Intel 4004 assembly text string (panics on invalid IR)
  - 8 unit tests + 1 doc-test

### Changed

- Added `codegen-core` to `[dependencies]` to enable the `CodeGenerator` trait implementation.

## [0.1.0]

- Initial Rust port of the Intel 4004 backend compiler.
