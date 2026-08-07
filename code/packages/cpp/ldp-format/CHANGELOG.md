# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C++17, header-only port of the Rust `ldp-format` crate in namespace
  `ca::ldp_format`: read/write of the LANG22 `.ldp` profile-artefact binary
  format (version 1) — header, deduplicated string table, and nested module →
  function → instruction records.
- `write` (deterministic, first-occurrence-order string interning) and `read`
  over plain value structs (`LdpFile`, `Header`, `ModuleRecord`,
  `FunctionRecord`, `InstructionRecord`, `TypeSeen`) with `operator==`.
  Exceptions (`Error` carrying an `ErrorKind`) replace the Rust `Result`.
- Untrusted-input safe: a bounds-checked `ByteReader` throws on truncation /
  bad string index / bad enum byte, and nested vectors grow incrementally rather
  than pre-allocating from untrusted counts. Verified clean under ASan + UBSan.
- 38 checks mirroring the crate's unit tests (empty/rich round-trip,
  determinism, string-table dedup, bad magic / version / truncation, language
  validation, unicode names, full enum coverage) run under every ISO C++
  compiler via the shared `iso-harness`.
