# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Added

- `record::encode`, a byte-compatible record writer with SQLite's minimal
  signed integer widths and self-referential header-length varints.

## [0.1.0] - 2026-07-13

### Added

- Header-only ISO C++17 port of the Rust `sqlite-file` crate in namespace
  `ca::sqlite_file`: a zero-dependency, read-only decoder for the SQLite
  on-disk format.
- Six layers — `varint::read`/`write`, `record::decode` (returns
  `std::optional<std::vector<Value>>`, `Value` a `std::variant` of
  Null/Int/Real/Text/Blob with faithful UTF-8-lossy text), `parse_header`, a
  zero-copy `Pager`, `walk_table`/`walk_index` with overflow-chain reassembly,
  and the schema layer (`read_schema`, `table_root_page`, `read_table`,
  `read_without_rowid_table`).
- Hardened against untrusted input: bounds-checked reads, an explicit b-tree
  stack with a visited-page set (cycle detection) and a running byte cap
  (amplification DoS). Where the Rust crate returns `Result`, this port throws
  a `SqliteError` carrying an `Error` code; `record::decode` returns
  `std::optional` to mirror the crate's `Option`.
- ~100k checks mirroring the crate's unit tests (varint golden vectors + a 50k
  round-trip sweep + truncation, record decode cases, header parsing, the
  pager, and b-tree table/index walks including overflow reassembly, cycle
  detection, and the amplification guard) run under every ISO C++ compiler via
  the shared `iso-harness`. Verified clean under ASan + UBSan.
