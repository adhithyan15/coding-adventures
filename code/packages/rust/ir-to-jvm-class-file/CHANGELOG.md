# Changelog

## [0.3.0] — 2026-05-11

### Added — Bitwise OR, XOR, NOT lowering + `validate_for_jvm`

- **`IrOp::Or`** — lowered to JVM `ior` (opcode `0x80`).
- **`IrOp::OrImm`** — register + immediate OR, lowered to `sipush` / `ldc` + `ior`.
- **`IrOp::Xor`** — lowered to JVM `ixor` (opcode `0x82`).
- **`IrOp::XorImm`** — register + immediate XOR, lowered to `sipush` / `ldc` + `ixor`.
- **`IrOp::Not`** — one's-complement bitwise NOT synthesised as `iconst_m1` + `ixor`
  (JVM has no native `inot`; this matches the WASM backend's `i32.const -1; i32.xor` pattern).
- **`validate_for_jvm(program) -> Vec<String>`** — public dry-run function: returns an empty
  list if the program is valid for JVM lowering, or a list of error strings otherwise.
  Mirrors the Python backend's `validate_for_jvm`.
- 8 new unit tests covering all five bitwise ops, `validate_for_jvm` success path, multi-op
  validation sweep, and the oversize-data error path.

### Changed

- Bumped version from `0.1.0` → `0.3.0` to align with `compiler-ir` (v0.2.0) and
  `ir-to-wasm-compiler` (v0.3.0) which landed the same bitwise-op additions.

---

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
