# Changelog

## Unreleased

### Fixed

Trimmed the `REM prereqs:` declaration in `BUILD_windows` so it lists exactly the
transitive prerequisites the dependency graph actually knows about
(`compiler-ir`, `ir-to-wasm-compiler`, `wasm-leb128`, `wasm-types`, `wasm-opcodes`),
matching the `PERL5LIB` chain in `BUILD`. It previously named thirteen packages that
are not dependencies of this one (`brainfuck`, `jit-core`, `vm-core`, `codegen-core`,
`compiler-source-map`, `grammar-tools`, `lexer`, `virtual-machine`, `interpreter-ir`,
`brainfuck-ir-compiler`, `wasm-module-encoder`, `wasm-module-parser`, `wasm-validator`),
which the build tool's BUILD validator rejects as undeclared local package refs.

## [0.1.0] - 2026-04-18

### Added

- Thin validation wrapper that reports lowering failures as protocol-friendly
  diagnostics.
- Coverage for valid IR and unsupported syscall failures.
