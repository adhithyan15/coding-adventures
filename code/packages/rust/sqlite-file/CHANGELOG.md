# Changelog — sqlite-file

## 0.15.0 - Unreleased

**Phase F (writer): tree growth — a table can now exceed one leaf.** When a
table's rows don't fit on a single leaf page, the writer splits them into
several data leaves under an **interior table b-tree page** (type `0x05`) that
becomes the table's root. `encode_table_pages` was generalised to
`encode_table_btree`, which returns the table's whole page list (root first):
it partitions the rowid-ordered cells into leaves by cumulative on-page
footprint, allocates the interior page + leaves (+ each leaf's overflow pages)
contiguously from the root page, and builds the interior divider cells
(`[u32-be left-child][varint largest-rowid-key]`) plus the right-most-child
pointer via the new `pack_interior_page`. A table that still fits on one leaf
keeps the byte-identical flat layout. Gates: an own-reader round-trip over ~300
rows (asserting the root is an interior `0x05` page) and a **real bundled-C
SQLite** cross-check — a 500-row table passes `PRAGMA integrity_check` (`"ok"`)
and reads back in full and in rowid order. Still **one interior level**: a table
with more leaves than fit on a single interior page is rejected with
`Unsupported` (multi-level trees are a later rung), as is page-1 `sqlite_schema`
overflow.

## 0.14.0 - Unreleased

**Phase F (writer): multi-table databases.** New
`page_writer::write_multi_table_db(page_size, &[TableSpec])` (where `TableSpec =
(name, create_sql, rows)`) emits a complete database holding **several** tables
in one call. Page 1 carries the DB header + a `sqlite_schema` leaf with one row
per table; pages 2… hold each table's data leaf followed by its overflow pages,
table by table, so each table roots wherever its leaf lands after the previous
tables' pages. `write_single_table_db` is now a thin wrapper over the one-table
case (byte-identical output). New internal helper `encode_table_pages` factors
out per-table leaf+overflow encoding. Gates: an own-reader round-trip across
three tables (one with an overflow row, to prove root-page allocation survives a
table that consumes extra pages), and a **real bundled-C SQLite** cross-check —
the multi-table file passes `PRAGMA integrity_check` (`"ok"`), `sqlite_schema`
lists every table, and each table's rows (including the overflow row) read back
over SQL. Still: one data leaf per table (no interior/tree growth) and the
combined `sqlite_schema` rows must fit on page 1's single leaf — later rungs.

## 0.13.0 - Unreleased

**Phase F (writer): overflow chains.** `page_writer::write_single_table_db` now
stores rows whose record exceeds the inline limit (`usable − 35`) by spilling the
tail into an **overflow-page chain**, instead of rejecting them with
`Unsupported`. The inline/overflow split mirrors the reader's
`split_and_reassemble` byte-for-byte (`M = ((U−12)·32/255) − 23`,
`K = M + (P−M) mod (U−4)`, inline `= K if K ≤ X else M`), and each overflow page
is `[u32-be next-page][content]` with `next = 0` on the last — exactly what
`btree::follow_overflow` expects. Overflow pages are allocated after the data
leaf (page 3 onward, in rowid order) and the DB-header page count is updated to
match. New helpers: `table_leaf_inline_len`, `build_leaf_cell`,
`encode_overflow_pages`; the leaf packer was split into a reusable
`pack_leaf_cells` shared by the inline-only `fill_table_leaf_page` and the
overflow-aware assembler (`encode_table_leaf_page` unchanged behaviourally).
Gates: an own-reader round-trip for a multi-page overflow row interleaved with
inline rows, plus a **real bundled-C SQLite** cross-check — our written file
passes `PRAGMA integrity_check` (`"ok"`) and reads back every row over SQL.
Still one data leaf only (no interior/tree growth) and the `sqlite_schema` row on
page 1 must still fit inline — those remain later rungs.

## 0.12.0 - Unreleased

**Phase F (writer): one-call whole-file assembler.** New
`page_writer::write_single_table_db(page_size, table_name, create_sql, rows)`
emits a complete, re-readable single-table SQLite database in one call — wiring
together `Header::encode`, `schema::table_schema_row`, `record::encode`, and the
leaf-page writer. Page 1 is the 100-byte DB header followed (at offset 100) by the
`sqlite_schema` leaf describing the table at root page 2; page 2 is the data leaf
holding the rows. The result reads straight back through `schema::read_table(&db,
name)`. Internally the leaf-page writer was refactored into a shared
`fill_table_leaf_page(page, header_offset, …)` so page 1 (b-tree header at offset
100) and ordinary pages (offset 0) share one code path; `encode_table_leaf_page`
is unchanged behaviourally. This turns the writer milestone's hand-assembly recipe
into a real API — the ergonomic capstone toward dropping the `rusqlite` dev-dep.
Round-trip tests: assemble a table, read it back by name; empty-table and
bad-page-size cases.

## 0.11.0 - Unreleased

**Phase F (writer), rung 4 — the `sqlite_schema` row + a full single-table file.**
`SchemaEntry::to_record_columns()` is the inverse of the reader's schema-row
decode: it serialises an entry into the five `sqlite_schema` columns
(`type`, `name`, `tbl_name`, `rootpage`, `sql`), and the convenience
`table_schema_row(name, root_page, sql)` builds the row for an ordinary rowid
table. Combined with the earlier rungs — `record::encode` (row payload, 0.8.0),
`page_writer::encode_table_leaf_page` (leaf page, 0.9.0), and `Header::encode`
(100-byte DB header, 0.10.0) — the writer can now emit a **complete, re-readable
single-table database**: a test assembles a two-page file by hand (page 1 = DB
header + a `sqlite_schema` leaf at offset 100 describing table `t` at root page 2;
page 2 = the data leaf) and reads the rows back through the real reader
(`schema::read_table` by table name) plus confirms the schema round-trips. This
completes the byte-level toolkit that makes dropping the `rusqlite` dev-dependency
viable. (A one-call whole-file assembler that handles page 1's 100-byte b-tree
offset is a natural follow-up.)

## 0.10.0 - Unreleased

**Phase F (writer), rung 3: the 100-byte DB-header encoder.** New
`Header::encode(&self) -> Vec<u8>` is the exact inverse of `Header::parse` —
`parse(encode(h)) == h`. It writes the magic string, page size (65536 stored as
the special value 1), reserved-space byte, the fixed 64/32/32 payload-fraction
constants SQLite requires, and every reader-surfaced field (change counter, page
count, freelist trunk/count, schema cookie/format, text encoding) at their exact
offsets; everything else is zero. Paired with `page_writer::encode_table_leaf_page`
(0.9.0) and `record::encode` (0.8.0), a caller can now lay down page 1's header +
a leaf page to build a file our own `Pager::open`/`Header::parse` reader accepts —
the write path is now three rungs deep toward emitting a re-readable single-table
`.sqlite` database (the milestone for eventually dropping the rusqlite dev-dep).
Round-trip tests cover a typical header, the 65536/reserved/UTF-16 edge cases, and
opening an encoded header through the pager.

## 0.9.0 - Unreleased

**Phase F (writer), rung 2: the table-leaf PAGE writer.** New
`page_writer::encode_table_leaf_page(page_size, reserved_space, cells)` packs a
set of `(rowid, record-bytes)` cells into one table b-tree leaf page (type
`0x0D`) — the byte-level inverse of the leaf reader in `btree`. It writes the
8-byte page header, the cell-pointer array (rowid order), and the cells packed
from the end of the page downward (`[payload-len varint][rowid varint][record
bytes]`), sorting cells by rowid and setting the cell-content-area start. One
capability only: a single leaf page, no overflow, no interior pages, no
freeblock coalescing. Records too large to store inline (would need an overflow
chain), duplicate rowids, more than 65535 cells, a page that cannot hold the
cells, and bad page sizes are all rejected with typed errors rather than
producing corrupt bytes. Round-trip gate: encoded pages (both raw cells and
`record::encode`-produced records, end-to-end) are read back through the real
`Pager::open` + `btree::walk_table` reader and asserted equal, in rowid order.

## 0.8.0 - Unreleased

**Phase F (writer) begins: the record ENCODER.** `record::encode(&[SqlValue])
-> Vec<u8>` is the exact inverse of `record::decode` — it serialises a row into
one byte-compatible SQLite record (header + payload), so `decode(encode(row))
== row` and a produced record drops straight into a table b-tree leaf cell. It
reuses the existing minimal `varint::write`, chooses the **shortest** integer
serial type SQLite would (0/1 inline for the values 0/1; 8/16/24/32/48/64-bit
by magnitude), and resolves the self-referential header-length varint by
widening until consistent. Verified with byte-for-byte golden vectors (the same
bytes the decoder tests assert on), a 20k-row `decode(encode(row)) == row`
sweep across every storage class, and a large-header case that forces a 2-byte
length varint. Next Phase-F slices: page/file-header writing, then emitting a
minimal single-page table-leaf `.sqlite` file real `sqlite3`/`rusqlite` can open.

## 0.7.0 - Unreleased

Phase E5 (cont.): the **name-based convenience reader** for `WITHOUT ROWID`
tables, so callers no longer need to hand-resolve a root page and pick the right
b-tree walker.

### Added

- **`read_without_rowid_table(bytes, name) -> Vec<Vec<SqlValue>>`** — the
  `WITHOUT ROWID` sibling of `read_table`. It resolves the table's root page from
  `sqlite_schema`, walks its **index** b-tree via `btree::walk_index`, and
  decodes each record into columns. There is no rowid, so it returns bare column
  vectors (not `(rowid, columns)`); the record holds every column in declared
  order, exactly like a rowid table.

## 0.6.0 - Unreleased

Phase E5: **index b-tree walking** — the raw-file primitive behind `WITHOUT
ROWID` tables (and, later, real index scans). `WITHOUT ROWID` tables store their
rows in an *index* b-tree keyed by the record itself, which the table-only
`walk_table` rejected (`unexpected b-tree page type`).

### Added

- **`btree::walk_index(pager, header, root_page) -> Vec<Vec<u8>>`** walks an
  index b-tree and returns every entry's record bytes. It handles the three ways
  index pages differ from table pages:
  - page types `0x0A` (leaf) / `0x02` (interior), not `0x0D` / `0x05`;
  - cells carry **no rowid** (`[len][payload]`, or `[child][len][payload]` on an
    interior page);
  - a SQLite index is a **true b-tree**, so a divider key on an interior page is
    a genuine entry stored only there — `walk_index` emits interior payloads as
    well as leaf payloads (a table b-tree copies rowids to the leaves, so
    `walk_table` never emits interior cells). Confirmed against real SQLite: an
    800-row `WITHOUT ROWID` table (a two-level index b-tree) round-trips to
    exactly its 800 records, none duplicated or missing.
  - overflow uses the **index** inline-split ceiling `X = ((U-12)*64/255) - 23`
    (smaller than the table ceiling `U - 35`).
  Bounds-, cycle-, and amplification-guarded exactly like `walk_table` — a
  corrupt tree yields `Err`, never a panic or unbounded work.
- Six unit tests (single leaf, empty, interior-emits-divider-plus-children,
  overflow reassembly, wrong-page-type rejection, child-cycle detection).

### Changed

- The table-leaf overflow reassembly was factored into a shared
  `split_and_reassemble` helper (parameterised by the inline ceiling) now used
  by both `walk_table` and `walk_index`; behaviour of the table path is
  unchanged (its 33 tests still pass).

## 0.5.0 - Unreleased

Phase E4: **`sqlite_schema` lookup** and the public **`read_table(bytes, name)`**
API. Callers no longer need to hand-walk page 1 to find a table root page before
reading rows.

### Added

- New `schema` module:
  - `read_schema(bytes)` decodes the five-column `sqlite_schema` rows into
    `SchemaEntry` values (`type`, `name`, `tbl_name`, `rootpage`, `sql`).
  - `table_root_page(bytes, name)` resolves a real table name to its root b-tree
    page and reports `SqliteError::NoSuchTable` for missing tables or views.
  - `read_table(bytes, name)` opens the pager, resolves the root page, walks the
    table b-tree, and returns rows as `(rowid, Vec<SqlValue>)` in rowid order.
- Root-level re-exports for the E4 API, so downstream Engram code can call
  `sqlite_file::read_table(...)` directly.

### Verified

- New `rusqlite` cross-checks confirm schema roots match SQLite's own
  `sqlite_schema` view and that `read_table` decodes named-table rows including
  INTEGER PRIMARY KEY aliases, REAL, BLOB, NULL, and an overflow-chain TEXT row.
- Full `cargo test --manifest-path code/packages/rust/sqlite-file/Cargo.toml`
  passes: unit tests, cross-check tests, and doc tests.

### Next

- Phase E5: cut `engram-anki-package`'s V11 collection reader over from
  `rusqlite` to `sqlite-file::read_table`.

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
