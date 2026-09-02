# sqlite-file — zero-dependency reader for the SQLite on-disk format

A from-scratch, pure-Rust reader for the subset of the [SQLite file
format](https://www.sqlite.org/fileformat2.html) that the Engram Anki-package
importer needs. It exists so the Engram stack can read `.anki2` collection
databases **without linking the third-party `rusqlite` crate** (which bundles
the entire C SQLite library behind an `unsafe` FFI boundary).

This is part of the Engram zero-dependency program
(`code/specs/engram-zero-dep-plan.md`, Phase E). The byte layout it tracks is
specified in `code/specs/storage-sqlite.md`.

## Why

Engram imports `.apkg` decks; inside each is a real SQLite database from which
Engram reads a few tables (`col`, `notes`, `cards`, `revlog`, `graves`). That
was the last thing forcing `rusqlite` (and bundled C SQLite) into the graph.
This crate replaces the **read** path with a small, auditable, dependency-free
byte parser, and now carries a **write** path too (Phase F) — enough to emit a
complete, re-readable database, which is what lets the Engram export drop
`rusqlite` entirely.

## Scope

Only what Engram's collections use:

- **Table** b-trees (leaf `0x0D` + interior `0x05`).
- **Index** b-trees (leaf `0x0A` + interior `0x02`), which is also how SQLite
  stores `WITHOUT ROWID` tables — see `walk_index` and
  `read_without_rowid_table`.
- **Overflow chains** for records too large for one page.
- Standard page sizes and **UTF-8** text (encoding = 1).
- **Writing** a whole database in one call (`page_writer::write_multi_table_db`):
  several tables, overflow chains, and multi-level b-trees. Whole-database emit
  only — there is no incremental update.

Out of scope: WAL, encryption, non-UTF-8 encodings, and in-place modification
of an existing file. **Writing** emits table b-trees only, so a database this
crate writes cannot yet represent a `WITHOUT ROWID` table it can read.

## Status

Built leaf-to-root; this is the foundation:

| Layer | Module | State |
|-------|--------|-------|
| varint (1–9 byte integers) | `varint` | ✅ read + write, golden-vector + sweep tests |
| record / serial types → `SqlValue` | `record` | ✅ decode, golden-row tests |
| DB header + in-memory pager | `header`, `pager` | ✅ header fields (incl. `user_version`) + zero-copy 1-based pages |
| table b-tree walk (leaf + interior) | `btree` | ✅ `walk_table(root)` → `(rowid, record)` |
| overflow chains (records spanning pages) | `btree` | ✅ reassembled inline + overflow; cycle/size guarded |
| index b-tree walk (`WITHOUT ROWID` tables) | `btree` | ✅ `walk_index(root)` → record bytes; interior divider keys emitted |
| `sqlite_schema` + `read_table(bytes, name)` | `schema` | ✅ read API |
| `read_without_rowid_table(bytes, name)` | `schema` | ✅ read API |

## Usage

```rust
use sqlite_file::read_table;

let rows = read_table(&db_bytes, "notes")?;
for (rowid, columns) in rows {
    // INTEGER PRIMARY KEY columns are stored as rowid and appear as NULL in
    // the record payload; use `rowid` when a table aliases it.
    println!("{rowid}: {columns:?}");
}
# Ok::<(), sqlite_file::SqliteError>(())
```

## Verification

Every layer is cross-checked against the real bundled-C SQLite (`rusqlite`, a
**dev-dependency only**) as an independent oracle: the gate builds genuine
`.sqlite` files and confirms this reader decodes the exact bytes SQLite wrote,
before the Anki importer is ever cut over. See `tests/cross_check_reader.rs`.

## Testing

```
cargo test -p sqlite-file
```
