# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C17 port of the Rust `jvm-class-file` crate: a conservative parser
  for a small subset of the JVM `.class` format, plus a minimal one-method
  class-file builder.
- Parses the class header, constant pool (Utf8/Integer/Long/Double/Class/String/
  Fieldref/Methodref/NameAndType, with Long/Double occupying two slots), fields,
  methods, and the `Code` attribute (with nested attributes). Resolvers
  (`jvm_get_utf8`, `jvm_resolve_constant`, `jvm_resolve_fieldref`,
  `jvm_resolve_methodref`) and method/code accessors.
- `jvm_build_minimal_class_file` + `jvm_build_params_default`, and
  `jvm_parse_class_file` / `jvm_class_free`.
- **Untrusted-input safety**: a bounds-checked reader with a sticky
  `JvmStatus` returns `JVM_ERR_FORMAT` (with a diagnostic) where the Rust code
  relies on panic-on-OOB slice indexing, so malformed input never reads out of
  bounds. All growable buffers guard `size_t` overflow. Verified clean under
  ASan + UBSan and the macOS `leaks` tool (0 leaks).
- 32 checks mirroring the crate's unit tests (round-trip build/parse, invalid
  magic, field/method-ref resolution, nested Code attribute, truncated input)
  run under every ISO C compiler via the shared `iso-harness`.
