# Mini-SQLite Polyglot Porting Specification

## Purpose

Python now has a working `mini-sqlite` facade that wires the SQL packages into
an embedded database API. This spec defines how to port that work across the
other languages in `code/packages` while keeping each PR reviewable and each
language useful before the full seven-package SQL pipeline exists everywhere.

The Python implementation is the reference behavior. The Python README and
tests are newer than `mini-sqlite-python.md` in one important respect: current
file-backed Python `mini-sqlite` uses the `storage-sqlite` backend and produces
SQLite-compatible `.db` files. Ports should follow the live code and tests when
they disagree with older prose.

## Supported Languages

The repository currently has package roots for these 17 languages:

| Language | SQL stack present today | Porting status |
| --- | --- | --- |
| Python | mini-sqlite, lexer, parser, backend, planner, optimizer, codegen, vm, storage-sqlite | Reference |
| TypeScript | lexer, parser, execution-engine, csv-source | First facade target |
| Go | lexer, parser, execution-engine, csv-source | Facade-ready |
| Ruby | lexer, parser, execution-engine, csv-source | Facade-ready |
| Rust | lexer, parser, execution-engine, csv-source | Facade-ready |
| Elixir | lexer, parser, execution-engine, csv-source | Facade-ready |
| Lua | lexer, parser, execution-engine | Facade-ready |
| Perl | lexer, parser, execution-engine | Facade-ready |
| C# | lexer, parser | Needs execution/storage foundations |
| F# | lexer, parser | Needs execution/storage foundations |
| Java | lexer, parser | Needs execution/storage foundations |
| Kotlin | lexer, parser | Needs execution/storage foundations |
| Haskell | lexer, parser | Needs execution/storage foundations |
| Wasm | lexer, parser wrappers | Needs host-facing facade decision |
| Dart | no SQL packages yet | Needs SQL foundations |
| Starlark | no SQL packages yet | Needs SQL foundations |
| Swift | no SQL packages yet | Needs SQL foundations |

## Target Package Contract

Every language should eventually expose a package named `mini-sqlite` (or the
language's idiomatic package/module spelling) with these concepts:

- `connect(database, options)` creates a connection.
- `":memory:"` creates an isolated in-memory database.
- File paths create a persistent database once the language has a storage
  backend. Until then, file paths must raise a typed "not supported" error.
- `Connection` owns transaction state and creates cursors or direct statement
  execution helpers.
- `Cursor` owns the active result set, metadata, row count, and fetch methods.
- Parameter binding starts with qmark placeholders (`?`) in every language.
- Errors use each language's idioms but preserve the Python categories:
  interface/programming, operational, data, integrity, internal, unsupported.

Where a language has a standard database interface, the facade should resemble
that interface. Where it does not, prefer the shape already used by the repo's
other packages: small constructors, plain value structs/classes, and explicit
typed errors.

## Feature Levels

### Level 0: Facade over existing SELECT engine

This is the fast path for TypeScript, Go, Ruby, Rust, Elixir, Lua, and Perl.
These languages already have a SELECT-oriented `sql-execution-engine`.

Level 0 must support:

- `connect(":memory:")`
- `CREATE TABLE [IF NOT EXISTS] name (...)`
- `DROP TABLE [IF EXISTS] name`
- `INSERT INTO name [(columns...)] VALUES (...)`
- `UPDATE name SET column = literal [, ...] [WHERE predicate]`
- `DELETE FROM name [WHERE predicate]`
- `SELECT ...` delegated to the existing execution engine
- qmark parameter binding for all statements
- `execute`, `executemany`, `fetchone`, `fetchmany`, `fetchall`
- simple transaction snapshots for `commit` and `rollback`

Level 0 may explicitly defer:

- file-backed persistence
- indexes and automatic indexes
- ALTER TABLE
- views, triggers, PRAGMA, CTEs, subqueries, windows, UDFs
- AST-level parameter binding when the local parser does not expose a stable AST
  mutation API yet

### Level 1: Full SQL pipeline

Languages graduate to Level 1 after they have ports of:

- `sql-backend`
- `sql-planner`
- `sql-optimizer`
- `sql-codegen`
- `sql-vm`

At Level 1, `mini-sqlite` should become a thin orchestrator like Python:
parse, bind, plan, optimize, compile, execute, and translate errors.

### Level 2: Persistent SQLite-compatible storage

Languages graduate to Level 2 after they have a storage backend equivalent to
Python's `storage-sqlite` package. File-backed `connect("app.db")` should then
write a database that can be read by the Python reference implementation and,
where practical, by SQLite tooling.

## Porting Order

1. Land this spec and the first TypeScript Level 0 package.
2. Port Level 0 facades to Go, Ruby, Rust, Elixir, Lua, and Perl.
3. Add shared conformance fixtures so every Level 0 facade runs the same SQL
   scripts and expected result sets.
4. Build missing execution/storage foundations for C#, F#, Java, Kotlin, and
   Haskell.
5. Decide whether Wasm is a standalone package or a compiled target for the
   Rust implementation.
6. Add SQL lexer/parser foundations for Dart, Starlark, and Swift before their
   facades.
7. Move languages from Level 0 to Level 1 and then Level 2 as foundations land.

## Conformance Tests

Every port should include a local test suite for the package plus a shared
script-based conformance suite once available.

The initial shared scenarios should cover:

- create table, insert rows, select all rows
- qmark binding in SELECT and INSERT
- projection, aliases, WHERE, ORDER BY, LIMIT/OFFSET
- aggregate SELECTs supported by the existing execution engines
- update/delete with WHERE predicates
- rollback restores pre-transaction state
- commit makes changes durable for the live connection
- wrong parameter counts raise programming errors
- unknown tables raise operational errors
- unsupported file-backed connections raise unsupported errors at Level 0

## PR Strategy

Keep PRs small enough to merge independently:

- One language facade per PR.
- Foundation packages before dependent facades.
- Shared conformance fixtures in their own PR when more than one language can
  consume them.
- If a facade depends on a foundation PR, base it on that branch and keep a
  timer/check-in loop watching for merge, conflicts, and CI failures.

The first PR created from this effort should include this spec and the
TypeScript Level 0 package so reviewers have both the roadmap and a concrete
implementation to inspect.
