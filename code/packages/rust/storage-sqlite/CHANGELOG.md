# Changelog — storage-sqlite

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
