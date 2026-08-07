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
byte parser. (Writing SQLite files — the reverse direction — is the separate,
larger Phase F.)

## Scope

Read-only, and only what Engram's collections use:

- **Table** b-trees (leaf `0x0D` + interior `0x05`) — no index b-trees.
- **Overflow chains** for records too large for one page.
- Standard page sizes and **UTF-8** text (encoding = 1).

Out of scope: writing, WAL, index b-trees, encryption, non-UTF-8 encodings.

## Status

Built leaf-to-root; this is the foundation:

| Layer | Module | State |
|-------|--------|-------|
| varint (1–9 byte integers) | `varint` | ✅ read + write, golden-vector + sweep tests |
| record / serial types → `SqlValue` | `record` | ✅ decode, golden-row tests |
| DB header + in-memory pager | `header`, `pager` | ✅ header fields + zero-copy 1-based pages |
| table b-tree walk (leaf + interior) | `btree` | ✅ `walk_table(root)` → `(rowid, record)` |
| overflow chains (records spanning pages) | `btree` | ✅ reassembled inline + overflow; cycle/size guarded |
| `sqlite_schema` + `read_table(bytes, name)` | `schema` | ✅ read API |

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
