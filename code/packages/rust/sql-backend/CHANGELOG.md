# Changelog

## 0.1.1 — Column-level COLLATE on `ColumnDef` + `SchemaProvider::column_collation`

- `ColumnDef` gains a `collation: Option<String>` field and a `.collation(name)`
  builder (BINARY normalises to `None`; NOCASE/RTRIM stored uppercased) so a
  `CREATE TABLE t(x TEXT COLLATE NOCASE)` sequence is persisted in the schema
  instead of being parsed and discarded.
- `SchemaProvider` gains `column_collation(table, column)` (default `None`),
  implemented by `BackendSchemaProvider` over `Backend::columns`. Lookups are
  case-insensitive; unknown table/column collapse to `Ok(None)` rather than an
  error. Consumed by sql-planner to let a bare `ORDER BY col` inherit the
  column's declared collation.

## 0.1.0

- Add the Rust sql-backend package with the portable backend contract and in-memory reference implementation.
