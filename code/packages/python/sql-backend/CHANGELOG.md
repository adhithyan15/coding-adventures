# Changelog

All notable changes to the `sql-backend` Python package are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.0] - 2026-04-27

### Added — Phase 4b: FOREIGN KEY constraints

- **`ColumnDef.foreign_key: object`** — optional `(ref_table, ref_col_or_None)`
  tuple typed as `object` to avoid circular import.  `compare=False, hash=False`
  preserves existing equality/hash behaviour.  `None` ref_col means "reference
  the parent's PRIMARY KEY".

## [0.5.0] - 2026-04-27

### Added — Phase 4a: CHECK constraints

- **`ColumnDef.check_expr: object`** — new optional field on the backend `ColumnDef`,
  typed as `object` to avoid a circular import with the planner's `Expr` hierarchy.
  `compare=False, hash=False` so existing equality and hash behaviour is unaffected.
  Carries the planner expression tree from the adapter layer through to codegen.

## [0.4.0] - 2026-04-27

### Added
- `ColumnAlreadyExists` error class — raised by `add_column` when the target column already exists.
- `Backend.add_column(table, column)` — new abstract method for ALTER TABLE ADD COLUMN support.
- `InMemoryBackend.add_column` — appends column to table schema and backfills existing rows with NULL.

## [0.3.0] - 2026-04-21

### Added

- **`current_transaction()` method on `Backend` ABC** — non-abstract default
  method that returns `None` for stateless backends.  Added so the VM can
  discover an already-open transaction handle when a fresh `_VmState` is
  created for a `COMMIT` or `ROLLBACK` that follows a `BEGIN` issued in an
  earlier `execute()` call.

- **`InMemoryBackend.current_transaction()`** — overrides the default to return
  `TransactionHandle(self._active_handle)` whenever a transaction is currently
  open, or `None` otherwise.  Makes multi-call transaction sequences (separate
  `execute()` calls for BEGIN, DML, COMMIT) transparent to the VM.

## [0.2.0] - 2026-04-20

### Added

- **Phase IX-2: index interface** — four new abstract methods on `Backend`:
  - `create_index(index: IndexDef) → None` — registers a named index on one or
    more columns of an existing table.  Raises `IndexAlreadyExists` on a
    duplicate name, `TableNotFound` for an unknown table, `ColumnNotFound` for
    an unknown column.
  - `drop_index(name, *, if_exists=False) → None` — removes a registered index.
    Raises `IndexNotFound` unless `if_exists=True`.
  - `list_indexes(table=None) → list[IndexDef]` — returns all registered
    `IndexDef` objects; when `table` is given, filters to that table only.
  - `scan_index(index_name, lo, hi, *, lo_inclusive, hi_inclusive) → Iterator[int]`
    — yields rowids in ascending key order within the given key-range bounds.
    Both `lo` and `hi` are `list[SqlValue] | None`; `None` means unbounded.
    Raises `IndexNotFound` for an unknown index name.
- **`IndexDef` dataclass** (`sql_backend.index`, re-exported from the root):
  - Fields: `name: str`, `table: str`, `columns: list[str]` (default `[]`),
    `unique: bool` (default `False`), `auto: bool` (default `False`).
  - The `auto` flag marks indexes created automatically by the advisor layer
    rather than explicitly by the user.
- **`IndexAlreadyExists` / `IndexNotFound`** — two new `BackendError` dataclass
  subclasses in `sql_backend.errors`:
  - `IndexAlreadyExists(index: str)` — `str()` → `"index already exists: '<name>'"`.
  - `IndexNotFound(index: str)` — `str()` → `"index not found: '<name>'"`.
  Both support equality comparison (same `index` value ↔ equal instances).
- **`InMemoryBackend`** fully implements all four index methods:
  - `_indexes: dict[str, IndexDef]` stores index definitions (B-tree is not
    materialised — `scan_index` does a linear scan so unit tests stay fast).
  - `scan_index` uses `_sql_sort_key(v)` for SQLite BINARY collation ordering:
    `NULL → (0, b"")`, `int/float → (1, v)`, `str → (2, v.encode())`,
    `bytes → (3, bytes(v))`. Guarantees NULL sorts before all other types.
  - Index snapshots included in `begin_transaction` / `rollback` so that
    `create_index` and `drop_index` are fully rolled back on abort.
- All new symbols exported from `sql_backend.__init__`:
  `IndexDef`, `IndexAlreadyExists`, `IndexNotFound`.

### Tests

- `tests/test_index_interface.py` — 46 new tests covering:
  - `IndexDef` construction, defaults, equality, flag preservation
  - `IndexAlreadyExists` / `IndexNotFound` error hierarchy and string formatting
  - `create_index` success, duplicate, bad table, bad column
  - `drop_index` success, missing, `if_exists` guard, double-drop
  - `list_indexes` all-tables and filtered views, creation-order preservation
  - `scan_index` full scan, NULL-sorts-first, equality lookup, range scan,
    exclusive lo/hi bounds, unbounded lo, ascending order, text ordering
  - Transaction rollback of `create_index` / `drop_index`; commit persistence
- Overall package coverage: **99.26%** (132 tests total).

## [0.1.0] - 2026-04-19

### Added

- Initial release. Pluggable data-source interface for the SQL query pipeline.
- `Backend` abstract base class with full read / write / DDL / transaction surface.
- Supporting value model: `SqlValue` (None | int | float | str | bool) plus
  `sql_type_name` / `is_sql_value` helpers.
- Supporting data model: `Row`, `RowIterator` protocol, `Cursor` protocol, and
  the `ListRowIterator` / `ListCursor` reference implementations.
- Schema model: `ColumnDef` with NOT NULL / UNIQUE / PRIMARY KEY flags plus
  `NO_DEFAULT` sentinel distinguishing "no default" from "default NULL".
- Error hierarchy: `BackendError` with six dataclass subclasses
  (`TableNotFound`, `TableAlreadyExists`, `ColumnNotFound`,
  `ConstraintViolation`, `Unsupported`, `Internal`).
- `SchemaProvider` minimal interface and `backend_as_schema_provider` adapter
  for use by the planner.
- `InMemoryBackend` reference implementation:
  - `from_tables` fixture helper for preloading schema and rows.
  - Constraint enforcement on insert and update (NOT NULL / UNIQUE /
    PRIMARY KEY implies NOT NULL + UNIQUE).
  - Default-value application for omitted columns.
  - Snapshot-and-restore transactions with stale-handle rejection.
  - Positioned UPDATE / DELETE via `ListCursor`.
- Shared conformance suite (`run_required`, `run_read_write`, `run_ddl`,
  `run_transaction`) plus `make_in_memory_users` golden fixture so every
  future backend is measured the same way.
- Unit tests covering values, errors, schema, iteration, InMemoryBackend,
  and the conformance suite itself.

