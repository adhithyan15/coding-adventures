# Mini-SQLite Level 0 Conformance Fixtures

A language-agnostic test fixture suite that every Level 0 `mini-sqlite` port must
pass. The fixtures are JSON files describing SQL operations and their expected
outcomes. Each port implements a small runner that loads and executes them.

## Purpose

Rather than re-writing the same 16 test scenarios in TypeScript, Go, Ruby, Rust,
Elixir, Lua, Perl, and every other language, this directory provides a single
shared specification. A conformance runner in each language reads `manifest.json`,
loads each fixture, executes the steps, and reports pass/fail.

## Fixture format

Each `fixtures/*.json` file contains:

```json
{
  "id": "...",
  "description": "...",
  "level": 0,
  "steps": [ ... ]
}
```

Steps have an `"op"` field selecting the operation. See `manifest.json` for the
full list of `op_types`.

### Common ops

| op | behaviour |
|---|---|
| `execute` | `connection.execute(sql, params?)` — no result checked |
| `executemany` | `connection.executemany(sql, param_seq)` |
| `query` | execute + `fetchall()`; compare `expected_columns` and `expected_rows` |
| `expect_error` | execute; assert an exception matching `error_type` is raised |
| `connect_expect_error` | `connect(database)`; assert `error_type` is raised |
| `commit` / `rollback` | call on the connection |
| `fetchone_test` | execute + two `fetchone()` calls; compare each row |
| `fetchmany_test` | execute + two `fetchmany(size)` calls; compare each batch |
| `fetchall_test` | execute + `fetchall()`; compare all rows |

### Type conventions

- `null` in JSON maps to the language's native null (`None`, `nil`, `null`, `Nothing`).
- Integer JSON values in `expected_rows` match language integers.
- Column name comparison is **case-insensitive** — `"ID"` and `"id"` both match.
- Row order matters only when `ORDER BY` appears in the SQL.

### Error type mapping

| Fixture `error_type` | Python | TypeScript | Go | Ruby | Rust | Elixir | Lua | Perl |
|---|---|---|---|---|---|---|---|---|
| `ProgrammingError` | `ProgrammingError` | `ProgrammingError` | `ErrProgramming` | `ProgrammingError` | `MiniSqliteError::Programming` | `ProgrammingError` | `ProgrammingError` | `ProgrammingError` |
| `OperationalError` | `OperationalError` | `OperationalError` | `ErrOperational` | `OperationalError` | `MiniSqliteError::Operational` | `OperationalError` | `OperationalError` | `OperationalError` |
| `NotSupportedError` | `NotSupportedError` | `NotSupportedError` | `ErrNotSupported` | `NotSupportedError` | `MiniSqliteError::NotSupported` | `NotSupportedError` | `NotSupportedError` | `NotSupportedError` |

## Running fixtures

### Python (reference)
```python
import json, pathlib
from mini_sqlite import connect

manifest = json.loads(pathlib.Path("manifest.json").read_text())
for path in manifest["fixtures"]:
    fixture = json.loads(pathlib.Path(path).read_text())
    # ... execute steps, assert expected_rows, assert error types
```

### Adding a language runner

Add a test file to the language's mini-sqlite package that:
1. Opens `manifest.json` (relative to the spec dir, or copy at build time).
2. For each fixture in `manifest["fixtures"]`, runs the steps.
3. Fails the test if any assertion doesn't match.

## Fixture index

| ID | Description |
|---|---|
| 01-create-select | CREATE TABLE, INSERT rows, SELECT all rows |
| 02-qmark-binding-insert | Qmark (?) parameter binding in INSERT and SELECT |
| 03-projection-aliases | Column projection and AS aliases |
| 04-where-filtering | WHERE with literals and qmark params |
| 05-order-by-limit-offset | ORDER BY, LIMIT, OFFSET |
| 06-aggregates | COUNT, SUM, AVG, MIN, MAX, GROUP BY, HAVING |
| 07-update-delete | UPDATE and DELETE with WHERE |
| 08-transaction-commit | COMMIT makes changes visible |
| 09-transaction-rollback | ROLLBACK restores pre-transaction state |
| 10-error-wrong-param-count | Wrong param count → ProgrammingError |
| 11-error-unknown-table | Unknown table → OperationalError |
| 12-error-file-path-level0 | File-backed connection → NotSupportedError (Level 0) |
| 13-drop-table | DROP TABLE, CREATE TABLE IF NOT EXISTS, DROP TABLE IF EXISTS |
| 14-executemany | executemany() batch operations |
| 15-fetchone-fetchmany | fetchone(), fetchmany(n), fetchall() |
| 16-null-handling | NULL in INSERT, SELECT, WHERE, IS NULL |

## Adding new fixtures

1. Create `fixtures/NN-description.json` following the existing format.
2. Add its path to `manifest.json`'s `"fixtures"` array.
3. All language runners pick it up automatically at next run.
