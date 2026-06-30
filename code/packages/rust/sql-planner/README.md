# sql-planner

Rust logical query planner for the Mini-SQLite Level 1 pipeline.

## Pipeline position

```text
sql-lexer → sql-parser → sql-planner → sql-optimizer → sql-codegen → sql-vm → mini-sqlite
```

This crate accepts a parsed SQL AST from `sql-parser` and a `SchemaProvider` (from `sql-backend`), and produces a `LogicalPlan` tree that describes *what* the query does without performing any I/O.

## What is a logical plan?

A logical plan is a tree of algebraic operators:

| Node | SQL construct |
|---|---|
| `Scan` | `FROM table` |
| `Filter` | `WHERE predicate` |
| `Join` | `JOIN … ON …` |
| `Aggregate` | `GROUP BY` + aggregate functions |
| `Having` | `HAVING predicate` |
| `Distinct` | `SELECT DISTINCT` |
| `Sort` | `ORDER BY` |
| `Limit` | `LIMIT … OFFSET …` |
| `Project` | `SELECT columns` (always outermost) |
| `Insert` | `INSERT INTO …` |
| `Update` | `UPDATE … SET …` |
| `Delete` | `DELETE FROM …` |
| `CreateTable` | `CREATE TABLE …` |
| `DropTable` | `DROP TABLE …` |

## Usage

```rust
use coding_adventures_sql_planner::{plan_sql, LogicalPlan};
use coding_adventures_sql_backend::{InMemoryBackend, backend_as_schema_provider};

let mut backend = InMemoryBackend::new();
// ... create tables ...
let schema = backend_as_schema_provider(&backend);

let plan = plan_sql("SELECT name FROM users WHERE age > 18", &schema)?;
println!("{plan:#?}");
```

## Plan node ordering

SQL's logical evaluation order differs from its textual order.  The planner
produces nodes in evaluation order (bottom-up):

```
Scan → Join → Filter → Aggregate → Having → Distinct → Sort → Limit → Project
```

`Project` is always the outermost (root) node because the SELECT list is
evaluated after sorting and pagination.

## Crate structure

```
src/
  lib.rs    — all types + planner + tests (single-file package)
```

## Running tests

```sh
# Windows
$env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER = "rust-lld"
cargo test --package coding-adventures-sql-planner

# Linux/macOS
cargo test --package coding-adventures-sql-planner
```
