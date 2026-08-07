# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C++17, header-only port of the Rust `jvm-class-file` crate in
  namespace `ca::jvm_class_file`: a conservative parser for a small subset of
  the JVM `.class` format, plus a minimal one-method class-file builder.
- `ClassFile` (with `std::vector<std::optional<ConstantPoolEntry>>` constant
  pool, `FieldInfo`/`MethodInfo` vectors, and `MethodInfo::code_attribute()`),
  resolvers mirroring the crate, `parse_class_file`, and
  `build_minimal_class_file` (+ `BuildMinimalClassFileParams`).
- Idiomatic C++ surface: exceptions (`Error : std::runtime_error`) where the
  Rust crate returns `Result`; a bounds-checked `ClassReader` cursor so no
  malformed input reads out of bounds. Verified clean under ASan + UBSan.
- 18 checks mirroring the crate's unit tests (round-trip build/parse, invalid
  magic, field/method-ref resolution, nested Code attribute, truncated input)
  run under every ISO C++ compiler via the shared `iso-harness`.
