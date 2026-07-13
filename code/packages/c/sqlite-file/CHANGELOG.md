# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C17 port of the Rust `sqlite-file` crate: a zero-dependency,
  read-only decoder for the SQLite on-disk format.
- Six layers — varint (`sf_varint_read`/`write`), record decode
  (`sf_record_decode` → `sf_value_t` Null/Int/Real/Text/Blob, with faithful
  UTF-8-lossy text), the 100-byte header (`sf_header_parse`), a zero-copy pager
  (`sf_pager_open`/`sf_pager_page`), table/index b-tree walks
  (`sf_walk_table`/`sf_walk_index`) with overflow-chain reassembly, and the
  schema layer (`sf_read_schema`, `sf_table_root_page`, `sf_read_table`,
  `sf_read_without_rowid_table`).
- Hardened against untrusted input: every read is bounds-checked, b-tree walks
  use an explicit stack (no recursion) with a visited-page bitmap (cycle
  detection) and a running byte cap (cell-aliasing amplification DoS); every
  allocation is size-overflow-checked and every error path frees its
  intermediates.
- Status-code API (`sf_error_t`, `SF_OK == 0`); decoded results are malloc-owned
  and freed with the matching `sf_*_free` routine.
- ~100k checks mirroring the crate's unit tests (varint golden vectors + a 50k
  round-trip sweep + truncation, record decode cases, header parsing, the
  pager, and b-tree table/index walks including overflow reassembly, cycle
  detection, and the amplification guard) run under every ISO C compiler via
  the shared `iso-harness`. Verified clean under ASan + UBSan and macOS
  `leaks`.
