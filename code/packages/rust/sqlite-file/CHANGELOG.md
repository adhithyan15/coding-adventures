# Changelog — sqlite-file

## 0.4.0 — Unreleased

Phase E3b: **overflow-chain reassembly** and the **full row round-trip gate**.
Rows too large for one page now read back in full, and the reader is measured
end-to-end against real SQLite for the first time.

### Added

- **Overflow-chain reassembly in `btree::read_leaf_cell`.** A record larger than
  the inline maximum keeps only its first *K* bytes on the leaf page, followed by
  a 4-byte overflow-page pointer; the rest lives in a linked list of overflow
  pages (`[u32-be next-page][content]`). The reader now stitches inline + overflow
  back into one contiguous record. The inline split follows SQLite's table-leaf
  rule exactly: `K = min_local + ((P − min_local) mod (usable − 4))`, clamped to
  `min_local` when it would exceed `usable − 35`. Replaces the previous
  `Unsupported("overflow chain")` bail-out.
- Overflow reassembly is bounded on every axis, so hostile input cannot hang or
  exhaust memory: a **visited-page set** turns any chain cycle into `Corrupt`, a
  chain that ends before the payload is complete is `Corrupt`, a payload claiming
  to be larger than the whole file is rejected up front, and the running total is
  capped at the file's byte length (the same anti-amplification discipline the
  leaf loop uses). `#![forbid(unsafe_code)]` throughout.

### Verified

- 6 new unit tests (single-page-spill reassembly, a multi-hop chain, chain-cycle
  detection, an early-terminated chain, an oversized-payload rejection, plus the
  retained aliasing-amplification guard).
- **The staged round-trip gate is now active.** `rows_round_trip_through_our_reader`
  builds a real `rusqlite` database whose table holds a row with 6.5 KB of TEXT
  (forcing overflow), walks `sqlite_schema` to find the table's root page, walks
  that table with our reader, decodes each record, and asserts every column equals
  what SQLite returns over `SELECT` — the overflow row included. Full suite green;
  clippy + fmt clean.

### Not yet included

- The **`read_table(bytes, name)`** convenience API (sqlite_schema lookup + walk
  in one call) is Phase E4; the Anki importer cutover is Phase E5.

## 0.3.0 — Unreleased

Phase E3a: the **table b-tree walk** — reads every row of a table given its
root page. Overflow-chain reassembly (for records too big for one page) and the
public `read_table(bytes, name)` API follow in E3b/E4.

### Added

- **`btree::walk_table(pager, header, root_page)`** — walks a table b-tree and
  returns `Vec<(rowid, record bytes)>` in rowid order. Handles **leaf** table
  pages (`0x0D`) and **interior** table pages (`0x05`, descending every child
  cell plus the right-most child in the page header), the page-1 offset-100
  quirk, and the cell-pointer array. Fully bounds- and cycle-checked: a corrupt
  tree (unexpected page type, cell pointer past the page, or a child-pointer
  cycle) returns a `SqliteError`, never a panic, out-of-bounds read, or infinite
  loop — the walk uses an explicit stack (no recursion) and a visited-page set.
- **`SqliteError::Corrupt(&str)`** — for structurally-inconsistent bytes a
  well-formed database never produces.

### Not yet included

- **Overflow chains**: a record larger than the inline maximum (`usable_size −
  35`) currently returns `Unsupported("overflow chain")` rather than truncating.
  Reassembly is Phase E3b, which also flips on the full row round-trip gate.

### Verified

- 5 new unit tests (single-leaf walk in rowid order, empty leaf, an interior
  page over two leaves, unknown-page-type reject, child-pointer-cycle detect).
  New cross-check: our reader walks the **real `sqlite_schema` b-tree** of a
  genuine `rusqlite`-built file and its `(name, rootpage)` set matches
  `SELECT name, rootpage FROM sqlite_schema` exactly — the first end-to-end row
  read against real SQLite. Full suite green (`#![forbid(unsafe_code)]`); clippy
  + fmt clean.

## 0.2.0 — Unreleased

Phase E2: the **database header** parser and a read-only **pager**, on the way
to the b-tree walk (E3) and the `read_table` API (E4).

### Added

- **`header`** — parse the 100-byte database header at the start of page 1:
  magic string, page size (with the `1 ⇒ 65536` convention and the power-of-two
  ≥ 512 validation), reserved-space-per-page, in-header page count, change
  counter, freelist trunk/count, schema cookie, schema format, and text
  encoding (`Utf8`/`Utf16Le`/`Utf16Be`). Exposes `usable_size()` = page size −
  reserved tail, the figure the b-tree/overflow math (E3) works against.
- **`pager`** — a zero-copy, read-only view over the database bytes: `Pager::open`
  parses the header and returns it alongside a `page(n)` accessor that borrows
  1-based page *n* as a sub-slice (page 1 includes the header bytes; the b-tree
  layer skips them). No journal, no cache — the database is already in memory.
  Bogus page numbers (0, past-EOF, or large enough to overflow the offset math)
  return `BadPageNumber` rather than panicking or reading out of bounds.
- **`error`** — a `SqliteError` enum (`BadMagic`, `Truncated`, `BadPageSize`,
  `BadPageNumber`, `Unsupported`) so every parse path is fallible on corrupt or
  hostile input. `#![forbid(unsafe_code)]` throughout.

### Verified

- 12 new unit tests (typical 4 KiB/UTF-8 header, the 65536 page-size quirk,
  reserved-space usable-size, bad magic / non-power-of-two size / short buffer /
  unknown encoding rejections, page-1-includes-header, page slicing, and
  out-of-range/overflow page numbers). The cross-check harness now parses a real
  `rusqlite`-built file's header with **our** `header` module and asserts every
  field matches an independent inline read of the same bytes, and that the pager
  agrees on page count and can return page 1. Full suite green; clippy + fmt clean.

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
