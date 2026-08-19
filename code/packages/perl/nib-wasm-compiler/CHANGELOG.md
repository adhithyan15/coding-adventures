# Changelog

## Unreleased

### Fixed

Trimmed the `REM prereqs:` declaration in `BUILD_windows` to the transitive
prerequisites the dependency graph actually knows about, mirroring the `PERL5LIB`
chain in `BUILD`. It previously also named `brainfuck`, `brainfuck-ir-compiler`,
`codegen-core`, `compiler-source-map`, `interpreter-ir`, `jit-core` and `vm-core`,
none of which this package depends on, which the build tool's BUILD validator
rejects as undeclared local package refs.

## [0.1.0] - 2026-04-18

### Added

- End-to-end Nib to Wasm orchestration for Perl.
- Structured package errors for parse, type-check, lowering, encoding, and
  write stages.
- Runtime-backed coverage for function calls, loops, and user-facing errors.
