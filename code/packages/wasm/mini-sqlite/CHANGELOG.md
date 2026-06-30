# Changelog

All notable changes to `mini-sqlite-wasm` will be documented here.

## [0.1.0] - 2026-06-30

### Added

- Initial Level 0 WebAssembly facade wrapping `coding-adventures-mini-sqlite`.
- `Connection` struct exported via `wasm-bindgen` with:
  - `new()` constructor — opens an in-memory database (`:memory:` only).
  - `execute(sql, params?)` — DDL/DML with optional JSON param array.
  - `executemany(sql, param_seq)` — batch DML with JSON array-of-arrays.
  - `query(sql, params?)` — SELECT returning `{"columns":[…],"rows":[[…],…]}` JSON.
  - `execute_for_fetch(sql, params?)` — SELECT that populates the cursor buffer.
  - `fetchone()` — next buffered row as JSON array string, or `null`.
  - `fetchmany(size)` — next `size` buffered rows as JSON array-of-arrays string.
  - `fetchall()` — all remaining buffered rows as JSON array-of-arrays string.
  - `commit()` / `rollback()` — snapshot-based transaction control.
- `open(database)` free function — alternative to the constructor for hosts where calling constructors is awkward.
- Error strings prefixed with the error-type name (`ProgrammingError: …`, `OperationalError: …`, `NotSupportedError: …`) so conformance runners can identify error kinds by prefix matching.
- 13 native (non-Wasm) unit tests covering DDL/DML, cursor fetch methods, transactions, NULL round-trips, error types, and DROP TABLE.
