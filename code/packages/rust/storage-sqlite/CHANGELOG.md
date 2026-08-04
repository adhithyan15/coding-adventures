# Changelog — storage-sqlite

## 0.4.0 — `WITHOUT ROWID` tables read end-to-end

`SqliteFileBackend::scan` now reads `WITHOUT ROWID` tables. These store their
rows in an *index* b-tree (keyed by the primary key, no rowid), which the
table-only read path rejected as `unexpected b-tree page type`.

- `scan` detects the `WITHOUT ROWID` clause (checked only in the text after the
  column-list parens, so a column named like the clause can't false-positive)
  and reads such tables via `sqlite_file::read_without_rowid_table` (built on
  `walk_index`) instead of `read_table`.
- The per-row assembly is factored into a shared `build_row` helper. A rowid
  table still materializes its `INTEGER PRIMARY KEY` column from the rowid; a
  `WITHOUT ROWID` table passes `None` for the rowid because the record already
  stores every column, including the primary key, directly. REAL affinity is
  applied on both paths.
- Verified end-to-end against real bundled SQLite (in mini-sqlite's
  `file_backed` tests): scalar / TEXT / composite primary keys, plus an 800-row
  table whose index b-tree spans interior pages.

## 0.3.0 — `list_indexes` reports the file's real indexes

`Backend::list_indexes` was a stub returning `Vec::new()`; it now reports the
index objects in the parsed `sqlite_schema` catalog (optionally filtered to one
table), so tools can introspect a real database's indexes the way `PRAGMA
index_list` does. The planner still full-scans (no `scan_index` yet).

- Explicit `CREATE INDEX` objects yield their `unique` flag and column list,
  recovered from the stored SQL: `parse_index_columns` takes the parenthesised
  column list (each column reduced to its bare identifier, so `col DESC` /
  `col COLLATE NOCASE` → `col`), and `index_is_unique` scans only the tokens
  before the column-list `(` and stops at `INDEX` (so a name containing
  "unique" is not misread).
- Auto-indexes (the ones SQLite creates to back `UNIQUE`/`PRIMARY KEY`) carry no
  catalog SQL; they are reported with `auto = true`, `unique = true`, and an
  empty column list (not recoverable from the catalog SQL).
- Verified by a new differential test (`mini-sqlite/tests/file_backed.rs::
  list_indexes_matches_real_sqlite`) that diffs the result — index names, unique
  flags, and column lists — against real SQLite's `PRAGMA index_list` /
  `PRAGMA index_info` over the same file, plus unit tests for the two parsers.

## 0.2.0 — Queryable `sqlite_master` / `sqlite_schema` catalog

`SELECT … FROM sqlite_master` (and its modern alias `sqlite_schema`) now works
over a real `.sqlite` file — applications introspect the database this way, so
the file backend must expose the schema catalog it already parses internally.

- `columns("sqlite_master")` returns the fixed five-column shape SQLite uses:
  `type text, name text, tbl_name text, rootpage integer, sql text`.
- `scan("sqlite_master")` yields one row per catalog object (tables, indexes,
  views, triggers) in on-disk (rowid) order — the same order SQLite returns.
  `rootpage` is `0` for objects with no b-tree (views/triggers), and `sql` is
  `NULL` only where SQLite stored none (e.g. auto-created indexes).
- Both names are matched case-insensitively. The catalog is **not** listed by
  `tables()` (nor by SQLite's `.tables`), but it is fully queryable — filters,
  projections, aggregates, and `ORDER BY` all run through the normal pipeline.
- Verified by a new differential test (`mini-sqlite/tests/file_backed.rs`,
  `sqlite_master_is_queryable_and_matches_real_sqlite`) that diffs mini-sqlite's
  answers against real bundled SQLite (`rusqlite`, dev-dep) over the same file,
  plus unit tests for the name matching and column shape.

## 0.1.0 — SqliteFileBackend (read-only)

First increment of Stream C / L4 in the mini-sqlite → full-SQLite-replacement
roadmap (`code/specs/mini-sqlite-full-conformance.md`): give the query engine a
file-backed storage engine so it can `SELECT` from a real `.sqlite` file.

### Added

- **`SqliteFileBackend`** — implements the `coding_adventures_sql_backend::Backend`
  trait over the zero-dependency `sqlite-file` reader. The three read methods a
  `SELECT` needs are real:
  - `tables()` — user table names from `sqlite_schema` (skips `sqlite_*`).
  - `columns(table)` — column names/types recovered by parsing the table's
    `CREATE TABLE` text (top-level comma split; table-level constraints skipped;
    quoted/bracketed/backtick identifiers handled).
  - `scan(table)` — every row as `Row = BTreeMap<String, SqlValue>`, mapping the
    file layer's values onto the engine's, and materializing `INTEGER PRIMARY KEY`
    columns from the rowid (the record stores `NULL` for them).
  - All mutating methods return `BackendError::Unsupported`; `list_indexes()` is
    empty; transactions are benign no-ops. `#![forbid(unsafe_code)]`.

### Verified

- Unit tests for the `CREATE TABLE` column parser (simple tables, nested-comma
  types like `DECIMAL(10, 2)`, table-level constraints, quoted identifiers, the
  `INTEGER` vs `INT` primary-key distinction) and the value mapping.
- `rusqlite` cross-check (dev-dependency only): builds genuine `.sqlite` files and
  asserts `tables()`, `columns()`, and `scan()` match what the real library
  reports over SQL — types, NULLs, and rowid aliases included.

### Next

- Wire mini-sqlite's `connect()` to open a file path through this backend
  (`Box<dyn Backend>`), so `SELECT` from a real database works end to end.
- Later: the byte-compatible writer (Phase F) makes this a read/write engine.
