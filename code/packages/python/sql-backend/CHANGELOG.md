# Changelog

All notable changes to the `sql-backend` Python package are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
