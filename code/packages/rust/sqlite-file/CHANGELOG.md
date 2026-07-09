# Changelog — sqlite-file

## 0.1.0 — Unreleased

Initial scaffold of a zero-dependency reader for the SQLite on-disk file format,
created to remove the third-party `rusqlite` crate (and its bundled C SQLite +
`unsafe` FFI) from the Engram Anki-package **read** path (Engram
zero-dependency program, `code/specs/engram-zero-dep-plan.md`, Phase E; byte
layout per `code/specs/storage-sqlite.md`).

### Added

- **`varint`** — the SQLite 1–9 byte big-endian variable-length integer used
  throughout the format. `read` (bounds-checked, returns `None` on truncation
  rather than panicking) and `write` (minimal-length encoding, needed later for
  the Phase F writer and for round-trip testing now). Verified by golden vectors
  from the format spec, a 50 000-value encode→decode round-trip sweep, and
  truncation cases.
- **`record`** — decode a record's bytes (header-length varint, serial-type
  varints, back-to-back payloads) into typed `SqlValue`s (`Null` / `Int` /
  `Real` / `Text` / `Blob`). Implements the full serial-type table including the
  zero-payload integers 0 and 1 (serial types 8/9), signed big-endian widths
  (1/2/3/4/6/8 bytes with sign extension), IEEE-754 f64, and even/odd BLOB/TEXT
  lengths. Corrupt or truncated records return `None` — no panics, no
  out-of-bounds reads. `#![forbid(unsafe_code)]`.

### Verified

- Cross-check harness (`tests/cross_check_reader.rs`) wired to the real
  bundled-C SQLite via `rusqlite` (**dev-dependency only** — no runtime link).
  It builds genuine `.sqlite` files and, at this layer, confirms the format
  constants the reader assumes (magic string, power-of-two page size, in-header
  page count vs. file length, schema format 1–4, UTF-8 text encoding). The
  full row-level round-trip gate is staged (`#[ignore]`) and turns on with the
  b-tree walk in Phase E3.

### Not yet included

- **Header + pager** (Phase E2), **table b-tree walk + overflow chains**
  (Phase E3), and the **`sqlite_schema` + `read_table(bytes, name)`** public API
  (Phase E4). Only after those does the Anki importer switch off `rusqlite`
  (Phase E5). Writing databases is Phase F.
