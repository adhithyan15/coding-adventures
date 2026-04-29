# Changelog

## [0.15.0] - 2026-04-28

### Added

**Runtime index maintenance (IX-12)**

Secondary indexes are now kept in sync with the table B-tree on every
DML operation, not only at `create_index` backfill time.  Combined with
the IX-11 UNIQUE flag, this means `CREATE UNIQUE INDEX` actually rejects
duplicate inserts at runtime — the IX-11 limitation note is gone.

- **`SqliteFileBackend.insert`** — now walks every index on the target
  table after the row is committed to the table B-tree, computing the
  index key from the inserted row and inserting `(key_vals, rowid)` into
  each index B-tree.  For UNIQUE indexes, a duplicate-key check (with
  NULL-distinct semantics) runs *before* any mutation so a violation
  leaves the database byte-for-byte unchanged.
- **`SqliteFileBackend.update`** — for each index, compares the old key
  (from the cursor's cached row) to the new key (from the proposed row).
  If they differ, deletes the old `(key, rowid)` entry and inserts the
  new one.  No-op for indexes whose columns weren't changed.  UNIQUE
  pre-check on the new key matches `insert` semantics.
- **`SqliteFileBackend.delete`** — removes the row's entry from every
  index before deleting the table B-tree row, using the cursor's cached
  row to recover the indexed column values.
- **`SqliteFileBackend._open_indexes_for(table)`** — new private helper
  that returns `[(name, columns, IndexTree)]` for every index on a
  table, with each `IndexTree` opened in the right unique mode (parsed
  from the stored `CREATE INDEX` SQL).

### Tests added

- `test_backend_index.py::TestRuntimeIndexMaintenance` — 9 tests:
  insert populates index, runtime UNIQUE violation, multiple NULLs
  allowed, delete removes index entry, update changes index entry,
  update of non-indexed column skips index, runtime UNIQUE on update,
  multiple-index maintenance, persistence across reopen.

### Notes

- This closes the IX-11 limitation where UNIQUE only fired at backfill.
  All UNIQUE checks now occur on every `INSERT`/`UPDATE` automatically.
- `INSERT` and `UPDATE` no longer auto-commit (unchanged behaviour); the
  caller — typically the SQL VM — must wrap DML in
  `begin_transaction`/`commit` to flush dirty pages.

## [0.14.0] - 2026-04-28

### Added

**UNIQUE index enforcement (IX-11)**

`IndexDef.unique` is now an enforced constraint at index level, not a reserved
field.  `CREATE UNIQUE INDEX` creates an index that rejects duplicate keys at
both backfill time and (eventually) at runtime insert/update time.

- **`IndexTree.create(..., is_unique=False)`** and **`IndexTree.open(...,
  is_unique=False)`** — new keyword.  When `True`, `IndexTree.insert(key,
  rowid)` raises `DuplicateIndexKeyError` if any existing entry shares the
  same key (independent of rowid).  The check runs **before** any disk write,
  so the tree is unchanged on rejection.
- **`IndexTree.is_unique` property** — exposes the flag for callers that need
  to introspect.
- **NULL semantics** — per SQLite, `NULL` values in a UNIQUE index are
  considered distinct from each other.  If any column in the key is `NULL`,
  the duplicate check is skipped.  This lets multiple rows have `NULL` in a
  UNIQUE column (single or composite).
- **`SqliteFileBackend.create_index(IndexDef(unique=True))`** — emits
  `CREATE UNIQUE INDEX` SQL into `sqlite_schema`, opens the index B-tree in
  unique mode, and runs the backfill.  If existing rows contain duplicates,
  the pending pager writes are rolled back and `ConstraintViolation` is
  raised.  The database is unchanged on failure.
- **`SqliteFileBackend.list_indexes`** — populates `IndexDef.unique` by
  parsing the stored `CREATE INDEX` SQL via the new `_parse_index_unique`
  helper.  The flag round-trips across close/reopen.
- **`_columns_to_index_sql(name, table, columns, *, unique=False)`** — gained
  a `unique` keyword that switches to `CREATE UNIQUE INDEX`.
- **`_parse_index_unique(sql)`** — case-insensitive regex match for
  `CREATE UNIQUE INDEX`.  Tolerates extra whitespace.

### Spec

- `code/specs/storage-sqlite-v3-auto-index.md` — added IX-11 to the phased
  build order, documented the storage and backend layer changes, and
  spelled out NULL-distinct semantics.  Removed the "UNIQUE deferred"
  non-goal.

### Tests added

- `test_index_tree.py::TestUniqueIndex` — 7 tests: default non-unique,
  distinct keys allowed, duplicate rejected, multiple NULLs allowed,
  composite NULL semantics, multi-leaf rejection, round-trip via reopen.
- `test_backend_index.py::TestUniqueIndex` — 7 tests: round-trip of unique
  flag, default non-unique, backfill rejection on duplicates, success on
  distinct data, NULL allowance, persistence across reopen, helper unit
  test.

### Limitations

- Index maintenance on runtime `insert` / `update` / `delete` is still a
  pre-existing gap in `SqliteFileBackend` — UNIQUE enforcement only fires
  at backfill time today.  Once index maintenance lands, runtime UNIQUE
  enforcement will work automatically with no further `IndexTree` changes.
- The duplicate scan inside `IndexTree.insert` uses `range_scan` which is
  O(N) worst case.  A dedicated O(log N) `_has_key` helper is a future
  optimisation.

## [0.13.0] - 2026-04-28

### Added

**Configurable page sizes (512–65536 bytes)**

Every layer of the storage stack now uses the actual page size read from the
database header rather than the module-level `PAGE_SIZE = 4096` constant.
Valid page sizes are the same powers-of-two that SQLite supports: 512, 1024,
2048, 4096, 8192, 16384, 32768, and 65536.

- **`Pager.create(page_size=…)`** — new keyword argument; defaults to 4096.
  Validates against the set of SQLite-valid sizes and raises `ValueError` for
  anything else.
- **`Pager.page_size` property** — returns `self._page_size`, an instance
  attribute set on construction / open instead of the module constant.
- **`Pager.open`** — reads the page size from bytes 16–17 of the database
  header on open (the standard SQLite field).  The encoded value 1 is decoded
  as 65536 per the SQLite spec.  An out-of-spec value raises
  `CorruptDatabaseError`.  Files without the SQLite magic bytes (e.g. raw
  pager-only test files) fall back to the default 4096.
- **`freelist.trunk_capacity(page_size)`** — new public function; computes
  the maximum leaf entries per trunk page for an arbitrary page size (was a
  module constant `TRUNK_CAPACITY = 1022` for 4096-byte pages only).  The
  constant is kept for backward compatibility.

### Changed

- **`Pager`** — `_open_file` now reads the SQLite magic and page-size field
  via a short-lived probe file handle that is closed before the main `_f`
  handle is opened.  This prevents Python's buffered I/O read-ahead from
  masking short-read detection after external file truncation (a behaviour
  difference observed on macOS).
- **`BTree`, `IndexTree`** — all module-level helper functions
  (`_local_payload_size`, `_cell_size_on_page`, `_read_ptrs`, …) gained a
  `page_size: int = PAGE_SIZE` parameter.  Class methods derive the page size
  from `self._pager.page_size` on every call.
- **`Freelist`** — `allocate` and `free` use `self._pager.page_size` and the
  new `trunk_capacity()` function.
- **`Schema` / `initialize_new_database`** — replaced the hard-coded `PAGE_SIZE`
  import with `pager.page_size` throughout.  `initialize_new_database` writes
  the correct page-size field into the SQLite header for any page size.

### Tests added

- `test_pager.py` — `test_create_with_1024_page_size`,
  `test_create_with_8192_page_size`, `test_create_rejects_invalid_page_size`,
  `test_open_reads_page_size_from_sqlite_header` (parametrised over all 7
  non-default valid sizes), `test_open_rejects_sqlite_header_with_bad_page_size`,
  `test_multi_page_roundtrip_small_pages`.
- `test_schema.py` — `TestNonDefaultPageSize`: initialize + create table +
  close + reopen round-trip at 1024 and 8192 bytes.

## [0.12.0] - 2026-04-28

### Added

**`SqliteFileBackend` — complete Backend interface coverage**

All previously-missing Backend interface methods are now fully implemented,
bringing the file backend to feature parity with `InMemoryBackend`.

- **`add_column(table, column)`** — implements ALTER TABLE ADD COLUMN by
  rewriting the stored `CREATE TABLE` SQL in `sqlite_schema` (via the new
  `Schema.update_table_sql` helper).  Existing rows are not touched on disk;
  `_decode_row` now returns the column's declared `DEFAULT` (or NULL) for
  columns absent from pre-existing record payloads, mirroring SQLite's own
  on-disk semantics.  Raises `ColumnAlreadyExists` for duplicate column names.

- **`current_transaction()`** — returns the active `TransactionHandle` or
  `None`, enabling multi-statement transaction sequences across separate
  `execute()` calls to retrieve the handle without external storage.

- **Savepoints (`create_savepoint`, `release_savepoint`,
  `rollback_to_savepoint`)** — implemented via an in-memory snapshot stack.
  `create_savepoint(name)` deep-copies the pager's dirty-page dict and records
  the current logical page count.  `rollback_to_savepoint(name)` restores both,
  re-attaches the schema object, and destroys savepoints created after the named
  one (keeping the named savepoint alive, per SQLite semantics).
  `release_savepoint(name)` drops the savepoint and all later ones without
  changing the data state.  Raises `Unsupported` for unknown savepoint names.

- **Triggers (`create_trigger`, `drop_trigger`, `list_triggers`)** — triggers
  are stored as `type='trigger'` rows in `sqlite_schema` with `rootpage=0`
  (the SQLite convention).  `create_trigger` serialises a `TriggerDef` to a
  `CREATE TRIGGER … BEGIN … END` statement.  `list_triggers(table)` parses
  them back into `TriggerDef` objects.  `drop_trigger(name, if_exists=False)`
  removes the row.  All three use new `Schema` helpers added below.

**`Schema` — new helpers**

- **`find_trigger(name)`**, **`list_triggers(table=None)`** — read `type='trigger'`
  rows from `sqlite_schema`, with optional per-table filtering.
- **`create_trigger(name, table, sql)`** — inserts a trigger row with `rootpage=0`.
- **`drop_trigger(name)`** — deletes the trigger row and bumps the schema cookie.
- **`update_table_sql(name, new_sql)`** — rewrites the `sql` field of an existing
  `type='table'` row in-place (using `BTree.update`), preserving the `rootpage`.

### Changed

- `_decode_row` now applies a column's declared `DEFAULT` (instead of unconditionally
  returning `NULL`) when a record's payload is shorter than the schema's column count.
  This handles rows written before an `add_column` call correctly.

## [0.11.0] - 2026-04-27

### Added
- `SqliteFileBackend.add_column` — raises `Unsupported("ALTER TABLE ADD COLUMN")`;
  file-format rewrite (updating sqlite_schema B-tree pages in-place) is not yet
  implemented.

## [0.10.0] - 2026-04-20

### Added

- **Phase IX-2: index interface on `SqliteFileBackend`** — implements the four
  index methods introduced by `sql-backend` 0.2.0 (`create_index`, `drop_index`,
  `list_indexes`, `scan_index`) for the file-backed SQLite engine.

  **`create_index(index: IndexDef) → None`**
  - Validates that the table and all listed columns exist; raises `TableNotFound`
    or `ColumnNotFound` on unknown names.
  - Raises `IndexAlreadyExists` if an index with the same name is already in
    `sqlite_schema`.
  - Generates a canonical `CREATE INDEX <name> ON <table> (<col>, ...)` SQL
    string and writes a new `type='index'` row to `sqlite_schema`.
  - Allocates a fresh `IndexTree` root page, then backfills all existing rows
    from the table's B-tree.
  - Commits pages to disk (`pager.commit()`).

  **`drop_index(name, *, if_exists=False) → None`**
  - Calls `Schema.drop_index(name)`, which frees the index B-tree pages via
    `IndexTree.free_all`, deletes the `sqlite_schema` row, and bumps the schema
    cookie.  Raises `IndexNotFound` unless `if_exists=True`.

  **`list_indexes(table=None) → list[IndexDef]`**
  - Scans `sqlite_schema` for `type='index'` rows, parses column names from
    the stored `CREATE INDEX` SQL via `_parse_index_columns`, and synthesises
    `IndexDef` objects.  Indexes whose names start with `auto_` have
    `IndexDef.auto=True`.

  **`scan_index(index_name, lo, hi, *, lo_inclusive, hi_inclusive) → Iterator[int]`**
  - Resolves the index root page from `sqlite_schema`, opens the `IndexTree`,
    and delegates to `IndexTree.range_scan`, yielding rowids.
  - Raises `IndexNotFound` for an unknown index name.

- **`Schema` index methods** (`schema.py`):
  - `Schema.create_index(name, table, sql) → int` — inserts a `type='index'`
    row, allocates a fresh `IndexTree` root, bumps the schema cookie, returns
    the root page number.
  - `Schema.drop_index(name)` — frees the index B-tree via `IndexTree.free_all`,
    deletes the schema row, bumps the cookie.  Raises `SchemaError` if the index
    does not exist.
  - `Schema.find_index(name) → (rowid, rootpage, sql) | None` — looks up an
    index by name in `sqlite_schema`.
  - `Schema.list_indexes(table=None) → list[tuple[str, str, int, str | None]]`
    — returns `(name, tbl_name, rootpage, sql)` tuples for all indexes,
    optionally filtered by `tbl_name`.

- **Helper functions** (`backend.py`):
  - `_parse_index_columns(sql)` — extracts the column name list from a
    `CREATE INDEX ... (<cols>)` SQL string; returns `[]` when `sql` is empty
    or unparseable.
  - `_columns_to_index_sql(name, table, columns)` — produces canonical
    `CREATE INDEX <name> ON <table> (<col>, ...)` SQL stored in `sqlite_schema`.

- **`pyproject.toml`**: added `[tool.uv.sources]` so that the local
  `../sql-backend` editable install is resolved instead of the PyPI registry.

### Tests

- `tests/test_backend_index.py` — 44 new tests covering:
  - `TestSchemaIndex`: `create_index` / `find_index` / `list_indexes` /
    `drop_index` at the `Schema` level
  - `TestBackendCreateIndex`: success, duplicate, bad table, bad column,
    `auto`/`unique` flag preservation, backfill of existing rows
  - `TestBackendScanIndex`: full scan, equality lookup, range scan with
    exclusive bounds, text ordering, orders by a non-PK column
  - `TestBackendDropIndex`: basic drop, `if_exists`, double-drop
  - `TestBackendListIndexes`: empty, all, filtered, after drop
  - `TestIndexPersistence`: index survives close + reopen; inserted rows
    after reopen are visible via `scan_index`
  - `TestOracleIndexVisible`: index created by `SqliteFileBackend` is
    visible to `sqlite3`; index created by `sqlite3` is readable by
    `scan_index`
- Overall package coverage: **95.62%** (555 tests total).

## [0.9.0] - 2026-04-20

### Added

- `storage_sqlite.index_tree` — phase IX-1: `IndexTree`, the index B-tree
  implementation using SQLite index page types (`0x0A` leaf, `0x02` interior).

  **`IndexTree`** stores `(key_vals, rowid)` pairs in ascending sort order
  using SQLite's default BINARY collation: NULL < INTEGER/REAL < TEXT < BLOB.
  Integers and floats compare numerically across types.

  **API:**
  - `IndexTree.create(pager, *, freelist=None) → IndexTree` — allocates a
    fresh root page and returns a new index tree.
  - `IndexTree.open(pager, rootpage, *, freelist=None) → IndexTree` — opens
    an existing index by root page number.
  - `insert(key, rowid)` — inserts a `(key, rowid)` pair.  Splits happen
    transparently at all tree levels (root-leaf split, non-root leaf split,
    interior page split, root interior split).  Raises `DuplicateIndexKeyError`
    for duplicate `(key, rowid)` pairs.
  - `delete(key, rowid) → bool` — removes the matching entry; returns `True`
    if found and removed, `False` if absent.
  - `lookup(key) → list[int]` — returns all rowids whose key equals `key`
    (supports non-unique indexes with multiple matching rowids).
  - `range_scan(lo, hi, *, lo_inclusive, hi_inclusive) → Iterator[...]` —
    yields `(key_vals, rowid)` pairs in ascending order within the given key
    range. `None` bounds mean unbounded.
  - `free_all(freelist)` — reclaims every page in the tree (used by
    `drop_index`).
  - `cell_count() → int` — total number of entries across all leaf pages.

  **Cell format (index leaf, type 0x0A):**
  ```
  [payload-size varint] [record bytes]
  ```
  The record encodes `[*key_cols, rowid]` as a standard SQLite record.  The
  rowid is the last column — this gives unambiguous sort order for non-unique
  indexes (matching SQLite's behaviour for non-UNIQUE indexes).

  **Cell format (index interior, type 0x02):**
  ```
  [left-child u32 BE] [separator-record bytes]
  ```
  The separator is the full `(key_cols, rowid)` composite key of the last
  entry in the left subtree — no ambiguity even with duplicate indexed values.

  **Comparison helpers (exported for testing and future use):**
  `_type_class`, `_cmp_values`, `_cmp_full_keys`, `_cmp_keys_partial`

- **`__init__.py`** exports: `IndexTree`, `IndexTreeError`,
  `DuplicateIndexKeyError`, `PAGE_TYPE_LEAF_INDEX`, `PAGE_TYPE_INTERIOR_INDEX`.

### Tests

- `tests/test_index_tree.py` — 80 tests covering:
  - Construction (`create`, `open`, `root_page`, `cell_count`)
  - Single-entry insert and lookup
  - Ordered scan (ascending sort order, rowid tiebreak)
  - Delete (present / absent, adjacent entries intact)
  - Duplicate-key lookup (non-unique indexes)
  - Range scan bounds (inclusive / exclusive lo / hi, empty ranges)
  - Splits (root-leaf split, interior splits, reverse-order inserts, 5 000 entries)
  - Deep splits with large keys (480-byte text keys trigger fast interior splits)
  - Value types (NULL, float, text, bytes, mixed-type ordering)
  - `free_all` with and without freelist
  - Comparison unit tests (`_cmp_values`, `_cmp_full_keys`, etc.)
  - Persistence (commit+reopen, rollback)
  - Error paths (oversized keys, unsupported types, edge cases)

## [0.8.1] - 2026-04-20

### Fixed

- **`_encode_row` / `_decode_row` byte-compatibility bug**: INTEGER PRIMARY KEY
  columns were previously *skipped* in the record payload.  Real SQLite instead
  writes a **NULL slot** for IPK columns (the actual integer is the B-tree cell
  key, not the payload), so any file written by the real `sqlite3` library would
  have an extra NULL at the start of every payload that our decoder was not
  consuming.  The result was a column shift: reading a sqlite3-written file with
  this backend would yield `{id: rowid, label: None, score: 'alpha'}` instead of
  `{id: rowid, label: 'alpha', score: 1.5}`.

  Fix: `_encode_row` now always appends `None` for each IPK column (matching
  sqlite3's output); `_decode_row` now consumes (and discards) the IPK slot from
  the decoded value list before mapping non-IPK columns, then injects the rowid
  for the IPK column as before.  Files written by earlier versions of this backend
  (which omitted the IPK slot) will decode incorrectly if opened by this version —
  they were never byte-compatible with real sqlite3, so this is a breaking change
  from 0.8.0 (still alpha).

- Updated `test_encode_skips_ipk` (renamed to `test_encode_ipk_as_null_placeholder`)
  and `test_decode_injects_rowid_for_ipk` in `tests/test_backend.py` to reflect
  the corrected encoding contract.

## [0.8.0] - 2026-04-20

### Added

- `storage_sqlite.backend` — phase 7: `SqliteFileBackend`, the
  `sql_backend.Backend` adapter that wires all lower-level layers (pager,
  freelist, schema, B-trees) behind the public Backend interface.

  **`SqliteFileBackend(path)`**

  Opens an existing SQLite database file or creates a new one (writing the
  100-byte database header and an empty `sqlite_schema` leaf on first open).
  Implements the full `sql_backend.Backend` ABC:

  - **`tables() → list[str]`** — returns table names in insertion order via
    `Schema.list_tables()`.
  - **`columns(table) → list[ColumnDef]`** — parses the `CREATE TABLE` SQL
    stored in `sqlite_schema` and returns column definitions.  Raises
    `TableNotFound` if the table does not exist.
  - **`scan(table) → RowIterator`** — opens a `_BTreeCursor` over the
    table's B-tree.  Rows are yielded in ascending rowid order (insertion
    order for tables without explicit rowid reuse).  Raises `TableNotFound`.
  - **`insert(table, row)`** — applies column defaults, enforces `NOT NULL`
    and `UNIQUE` / `PRIMARY KEY` constraints, chooses the rowid (using the
    `INTEGER PRIMARY KEY` value when present, otherwise `max + 1`), encodes
    the row as a SQLite record, and calls `BTree.insert`.  Raises
    `TableNotFound`, `ColumnNotFound`, `ConstraintViolation`.
  - **`update(table, cursor, assignments)`** — merges assignments into the
    current row, re-encodes, and calls `BTree.update` at the cursor's rowid.
    Enforces `NOT NULL` on the new values.  Raises `ConstraintViolation`,
    `ColumnNotFound`, `Unsupported` (non-native cursor or no current row).
  - **`delete(table, cursor)`** — calls `BTree.delete` at the cursor's
    rowid and clears the cursor's current-row state.  Raises `Unsupported`
    (non-native cursor or no current row).
  - **`create_table(table, columns, if_not_exists)`** — serialises the
    column list to `CREATE TABLE` SQL, inserts the schema row, and allocates
    a root page.  Raises `TableAlreadyExists` when `if_not_exists=False` and
    the table already exists; silently returns when `if_not_exists=True`.
  - **`drop_table(table, if_exists)`** — frees all pages in the table's
    B-tree, removes the schema row.  Raises `TableNotFound` when
    `if_exists=False` and the table is missing.
  - **`begin_transaction() → TransactionHandle`** — records an opaque
    handle; writes continue to accumulate in the pager's dirty-page table.
    Raises `Unsupported` if a transaction is already open.
  - **`commit(handle)`** — fsyncs dirty pages to disk via `pager.commit()`.
  - **`rollback(handle)`** — discards dirty pages via `pager.rollback()` and
    reattaches the `Schema` so post-rollback reads see the committed state.
  - **`close()`** — rolls back any open transaction, then closes the pager.
  - **Context-manager protocol** (`with SqliteFileBackend(path) as b:`):
    `__exit__` calls `close()`, so uncommitted writes are rolled back
    automatically on normal or exceptional exit.

  **`_BTreeCursor`** — internal class implementing both `RowIterator` and
  `Cursor` protocols.  Wraps a `BTree.scan()` generator; `next()` decodes
  each record; `current_row()` returns the last decoded row; `close()`
  stops iteration.  The backend uses the stored `_current_rowid` for
  positioned `update` and `delete`.

  **SQL helper functions** (module-level, used internally):

  - `_format_literal(value)` — Python value → SQL literal string.
  - `_columns_to_sql(table, columns)` — `list[ColumnDef]` → `CREATE TABLE`
    SQL string (parseable by both this module and the real `sqlite3` CLI).
  - `_tokenize(sql)` — lightweight regex tokeniser (strips comments).
  - `_parse_literal(tok)` — SQL literal token → Python value.
  - `_split_column_defs(body)` — comma-split respecting parenthesis depth.
  - `_parse_one_column(col_sql)` → `ColumnDef | None`.
  - `_sql_to_columns(sql)` — `CREATE TABLE` SQL → `list[ColumnDef]`.
  - `_is_ipk(col)` — returns `True` for `INTEGER PRIMARY KEY` columns.
  - `_encode_row(rowid, row, columns)` — encodes a row as a SQLite record
    payload, skipping IPK columns.
  - `_decode_row(rowid, payload, columns)` — decodes a record payload,
    injecting `rowid` for IPK columns.
  - `_find_max_rowid(tree)` — full scan to find the current max rowid.
  - `_choose_rowid(row, columns, tree)` — picks the rowid for a new row.
  - `_apply_defaults(row, columns)` — fills absent columns from defaults.
  - `_check_not_null(table, row, columns)` — raises `ConstraintViolation`.
  - `_check_unique(table, row, columns, tree, ...)` — full-scan uniqueness
    check; `NULL` values never conflict.

  **`SqliteFileBackend`** and the `_BTreeCursor` class (via `SqliteFileBackend`)
  are now exported from the package root.

- `pyproject.toml` updated: `dependencies = ["coding-adventures-sql-backend"]`.
- `BUILD` updated: installs `../sql-backend` before the package itself.
- Pass all four tiers of `sql_backend.conformance` (required, read-write,
  DDL, transactions) — 67 new backend tests, 431 total, 96% coverage.

## [0.7.0] - 2026-04-20

### Added

- `storage_sqlite.schema` — phase 6: `sqlite_schema` catalog table (CREATE TABLE /
  DROP TABLE / schema cookie management).
  - **`initialize_new_database(pager)`**: sets up a brand-new SQLite-compatible
    database file. Allocates page 1, writes the 100-byte database header (via
    `Header.new_database()`), and initialises an empty `sqlite_schema` table-leaf
    page at byte offset 100. Returns a `Schema` ready for DDL. The caller must call
    `pager.commit()` to persist the result.
  - **`Schema` class**: access layer for the `sqlite_schema` catalog B-tree that
    lives on page 1 (with its B-tree header at offset 100).
    - `Schema(pager, freelist=None)` — opens the schema tree on an existing
      database. The optional `Freelist` is forwarded to the B-tree so that
      `create_table` / `drop_table` allocate from and return pages to the freelist.
    - **`list_tables() → list[str]`**: full scan of `sqlite_schema`, returning the
      names of all `type = 'table'` rows in ascending rowid order (i.e. insertion
      order).
    - **`find_table(name) → (rowid, rootpage, sql) | None`**: looks up a table by
      name and returns the three fields needed for DML (`rootpage`) and
      round-tripping DDL (`sql`). Returns `None` when the table does not exist.
    - **`rootpage_for(name) → int | None`**: convenience wrapper that returns only
      the root page number (used when opening a table's B-tree for DML).
    - **`get_schema_cookie() → int`**: reads the u32 schema cookie at byte offset
      40 of the page-1 database header. The cookie is incremented on every CREATE
      or DROP TABLE so that clients can detect schema changes without re-reading the
      whole catalog.
    - **`create_table(name, sql) → int`**: validates the name is not already taken,
      allocates a fresh root page via `BTree.create`, inserts the five-column
      `sqlite_schema` row (`type='table'`, `name`, `tbl_name=name`, `rootpage`,
      `sql`), bumps the schema cookie, and returns the root page number.  Raises
      `SchemaError` if a table with the same name already exists.
    - **`drop_table(name)`**: locates the `sqlite_schema` row for *name*, calls
      `BTree.free_all(freelist)` to reclaim every page in the table's B-tree
      (interior pages, leaf pages, overflow chains), deletes the schema row, and
      bumps the schema cookie.  Without a freelist the root page is zeroed to
      prevent stale data from persisting.  Raises `SchemaError` if the table does
      not exist.
  - **`SchemaError(StorageError)`**: new exception class for schema-level errors
    (duplicate table name, unknown table).  Exported from the package root.
- **`BTree.free_all(freelist)`** — new method (added to `btree.py` for phase 6):
  reclaims every page in a B-tree via a depth-first post-order traversal.
  - **`BTree._free_subtree(pgno, hdr_off, visited)`**: internal helper that reads
    the page header, frees all overflow chains on leaf pages, recursively frees
    children of interior pages, then frees the page itself.  Page 1 is never freed
    (it is the database header page).  A `visited` set prevents double-frees on
    corrupt databases with pointer cycles.
- **`Schema` and `SchemaError` and `initialize_new_database`** are now exported from
  the package root (`storage_sqlite.__init__`).

## [0.6.0] - 2026-04-20

### Added

- `storage_sqlite.freelist` — phase 5: SQLite trunk/leaf freelist for page reuse.
  - **`Freelist` class**: wraps a `Pager` and manages the SQLite freelist via the
    two header fields at offsets 32 (`first_trunk`) and 36 (`total_pages`) of page 1.
  - **`Freelist.free(pgno)`**: returns a page to the freelist.  If the current trunk
    has room (fewer than `TRUNK_CAPACITY` = 1 022 leaf entries), *pgno* is appended
    as a leaf entry.  If the trunk is full or absent, *pgno* is promoted to a new
    trunk page that points to the old trunk as its successor.
  - **`Freelist.allocate()`**: pops a page from the freelist (LIFO — last leaf in
    the current trunk) and zero-fills it before returning.  If the current trunk has
    no leaf entries, the trunk page itself is returned and `first_trunk` advances to
    the next trunk.  Falls back to `Pager.allocate()` when the freelist is empty.
  - **`Freelist.total_pages`** property: reads the total count from the page-1 header.
  - **`TRUNK_CAPACITY`** module constant (1 022): maximum leaf entries per trunk page
    for 4 096-byte pages — exported for testing and documentation.
- `BTree` now accepts an optional `freelist` keyword argument in `__init__`,
  `create`, and `open`.
  - **`BTree._allocate_page()`**: new internal helper that calls
    `freelist.allocate()` when a freelist is injected, otherwise falls back to
    `pager.allocate()`.  Replaces all direct `pager.allocate()` calls inside the
    B-tree.
  - **`BTree._free_page(pgno)`**: new internal helper that calls `freelist.free(pgno)`
    when a freelist is injected.  Without a freelist, the page is zeroed in the
    pager's dirty table (backwards-compatible with phases 1–4).
  - **`BTree._free_overflow`** updated: now calls `_free_page()` for each overflow
    page in the chain instead of directly zeroing via `pager.write()`.  With a
    freelist, deleted overflow pages are reclaimed for reuse; without one the
    behaviour is unchanged.

### Fixed

- **`Pager.commit()` cache coherency bug**: after a successful commit the LRU page
  cache could hold stale pre-commit values for pages that were read before being
  dirtied in the same session.  The fix promotes every dirty page into the cache
  before clearing the dirty table, so reads immediately after commit see the
  committed data without going back to disk.  This bug had no effect on existing
  phases (all tests closed and reopened the pager after commit) but was exposed by
  the new freelist tests that commit mid-session and then re-read the header.
- **`Pager.rollback()` cache consistency**: dirty pages are now evicted from the
  cache before the dirty table is cleared, so reads after rollback fall through to
  the main file (which holds the correct pre-txn state) rather than potentially
  returning stale cached data from the aborted transaction.

## [0.5.0] - 2026-04-20

### Added

- `storage_sqlite.btree` — phase 4b: full recursive leaf and interior splits.
  - **Non-root leaf split**: `BTree.insert` now splits any full non-root leaf
    page into two halves, rewrites the existing page with the left half,
    allocates a right sibling, and calls `_push_separator_up` to propagate
    the separator key up the ancestor path. Trees can grow to arbitrary depth.
  - **`_push_separator_up`**: recursively inserts a new separator cell into a
    parent interior page.  If the parent is also full it is split via
    `_split_interior_page` (non-root) or `_split_root_interior` (root), and
    the process repeats with the grandparent.
  - **`_split_interior_page`**: splits a non-root interior page by removing
    the median cell, rewriting the existing page with the left half (rightmost
    child = median.left_child), and allocating a right sibling for the right
    half (rightmost child = original rightmost_child).  Returns
    `(median_sep_rowid, right_pgno)` for the caller to push further up.
  - **`_split_root_interior`**: splits the root interior page when it is full.
    Allocates two new interior children for the left and right halves and
    rewrites the root with a single separator cell.  The root page number never
    changes.
  - **`_find_leaf_with_path`**: extended traversal that records the full
    ancestor path `(pgno, hdr_off, chosen_idx)` from root to leaf's parent.
    `_find_leaf_page` is now a thin wrapper that discards the path.
  - **`_write_interior_page`**: helper that builds an interior page from scratch
    given a sorted cell list and a rightmost-child pointer.
  - **`_interior_cells_fit`** (module-level): checks whether a list of interior
    cells fits within the usable space of an interior page at a given
    `header_offset`, used by `_push_separator_up` to decide whether to split.
  - **`_split_leaf`**: splits a non-root leaf page in-place (rewrites existing
    page with left half, allocates right sibling with right half).
  - `PageFullError` is retained in the public API but is no longer raised by
    `BTree.insert` during normal operation.  Recursive splits handle all
    leaf-overflow cases transparently.

### Changed

- `insert` now uses `_find_leaf_with_path` instead of `_find_leaf_page` so
  that the ancestor path is available when a non-root leaf is full.
- Module docstring updated to document the interior split algorithm, the new
  helper methods, and the v1 limitation that orphaned overflow pages from
  splits are not reclaimed until phase 5 (freelist).

## [0.4.0] - 2026-04-20

### Added

- `storage_sqlite.btree` — phase 4a: interior page traversal + root-leaf split.
  - **Interior page support**: `_read_hdr` now returns `"rightmost_child"` for
    interior pages (type `0x05`). New helpers `_write_interior_hdr`,
    `_read_interior_ptrs`, `_read_interior_cell`, `_interior_cell_encode` expose
    the full interior-page format for callers and tests.
  - **Root-leaf split**: `BTree.insert` automatically promotes the root page from
    a leaf to an interior page when the leaf fills up, distributing cells across
    two freshly allocated child pages. The root page number never changes.
  - **Multi-level traversal**: `find`, `scan`, `delete`, `update`, and
    `cell_count` all traverse interior pages to reach the correct leaf. `scan`
    performs a full left-to-right DFS over all leaves, yielding rows in ascending
    rowid order.
  - **Cycle and depth guards** in all traversal paths: `_find_leaf_page` uses a
    depth counter (limit `_MAX_BTREE_DEPTH = 20`); `_scan_page` uses a visited
    set; both raise `CorruptDatabaseError` on corrupt trees.
  - **Child-pointer validation** in `_find_leaf_page` and `_scan_page`: any
    child pointer of 0 or beyond the pager's current page count raises
    `CorruptDatabaseError` before dereferencing.
  - `PageFullError` is now only raised for *non-root* leaf overflows; root-leaf
    overflow is resolved silently by the split. Phase 4b will extend splitting to
    non-root leaves.
  - `free_space()` updated to account for the wider 12-byte interior header when
    the root is an interior page.
  - `header_offset=100` (page-1 support) correctly preserved through the
    root-split rewrite (the 100-byte SQLite database header prefix is not zeroed).

## [0.3.0] - 2026-04-20

### Added

- `storage_sqlite.btree` — table B-tree leaf pages (phase 3).
  - `BTree.create(pager)` allocates and initialises a fresh empty leaf page.
  - `BTree.open(pager, root_page)` attaches to an existing page.
  - `insert(rowid, payload)` — sorted insert; raises `DuplicateRowidError` on
    collision, `PageFullError` when the leaf is full (splits land in phase 4).
  - `find(rowid)` — binary search over the sorted cell-pointer array; returns
    raw record bytes or `None`.
  - `scan()` — iterator over `(rowid, payload)` in ascending rowid order.
  - `delete(rowid)` — removes the cell and compacts the page in-place.
  - `update(rowid, payload)` — delete + re-insert; handles payload size changes.
  - Overflow chains: records larger than `max_local` (4 061 bytes for 4 096-byte
    pages) spill to linked overflow pages per the SQLite formula; reads and
    writes follow the chain transparently.
  - `header_offset=100` support for page 1 (database header occupies bytes 0–99).
  - `BTreeError`, `PageFullError`, `DuplicateRowidError` exported from package root.

## [0.2.0] - 2026-04-19

### Added

- `storage_sqlite.varint` — SQLite's 1..9 byte big-endian varint codec.
  `encode(value)`, `decode(data, offset)`, `encode_signed`, `decode_signed`,
  `size(value)`. Produces the shortest form for every value (required for
  byte-compat round-trip). Full u64 range supported; signed helpers span
  the i64 range via two's complement.
- `storage_sqlite.record` — record codec mapping row values ↔ bytes.
  Supports the nine value-bearing serial types (NULL, int8/16/24/32/48/64,
  float64 BE, constant-0, constant-1), plus BLOB (even serial types ≥ 12)
  and TEXT (odd serial types ≥ 13). Encoder picks the *smallest* serial
  type for every integer — matching sqlite3's byte-compat behaviour, so a
  record of `[None, 7, "hi"]` encodes to the exact same bytes sqlite3
  writes. Decoder rejects reserved serial types (10, 11), truncated
  payloads, and inconsistent header lengths.
- `Value` type alias exported at the package root for record values.

## [0.1.0] - 2026-04-19

### Added

- Initial release — phase 1 of the SQLite byte-compatible file backend.
- `storage_sqlite.header.Header` — dataclass for the 100-byte SQLite
  database header. Field-by-field layout per the v3 spec; `from_bytes`
  parses, `to_bytes` serialises, `new_database` constructs a fresh header
  with the conventional defaults (page size 4096, UTF-8, schema format 4).
  Validates magic string, page size (power-of-two, 512..65536), and text
  encoding on parse.
- `storage_sqlite.pager.Pager` — page-at-a-time I/O against a file, with:
  - 1-based page numbers (page 0 is never read or written).
  - Fixed page size of 4096 in v1.
  - Small LRU page cache (default 32 pages).
  - `allocate()` to claim a fresh page number at the file tail.
  - Rollback journal at `<db>-journal`: on first write to a page within a
    transaction, the original page contents are copied to the journal.
    `commit()` fsyncs the journal, applies staged writes to the main file,
    fsyncs the main file, then deletes the journal. `rollback()` discards
    the staged writes and the journal.
  - Context-manager protocol (`with Pager.open(path) as pager:`).
  - Crash-recovery on `open`: if a hot journal is present, its contents are
    replayed back into the main file before the pager becomes usable.

