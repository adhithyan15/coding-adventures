# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C17 port of the Rust `ldp-format` crate: read/write of the LANG22
  `.ldp` profile-artefact binary format (version 1) — a 32-byte header, a
  deduplicated string table, and nested module → function → instruction records.
- `ldp_write` (deterministic, first-occurrence-order string interning),
  `ldp_read`, `ldp_file_free`, and `ldp_file_equal` over a transparent nested
  data model. An `LdpStatus` enum replaces the Rust `Result`/typed errors.
- **Untrusted-input safety**: a bounds-checked reader returns
  `LDP_ERR_UNEXPECTED_EOF` / `LDP_ERR_BAD_STRING_INDEX` where the Rust code
  relies on typed errors; nested arrays grow incrementally rather than
  pre-allocating from untrusted counts (an improvement over the crate's
  `Vec::with_capacity`); all growable buffers guard `size_t` overflow. Verified
  clean under ASan + UBSan and the macOS `leaks` tool (0 leaks), including a fuzz
  sweep over every truncation and single-byte corruption of a rich file.
- 66 checks mirroring the crate's unit tests (empty/rich round-trip,
  determinism, string-table dedup, bad magic / version / truncation, language
  validation, unicode names, full enum coverage) run under every ISO C compiler
  via the shared `iso-harness`.
