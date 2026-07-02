# coding-adventures-sql-vm

Stack-machine bytecode VM for the Mini-SQLite SQL processing pipeline (Level 1).

## Position in the pipeline

```
sql-lexer → sql-parser → sql-planner → sql-optimizer → sql-codegen → sql-vm → mini-sqlite
```

`sql-vm` is the **sixth stage**.  It takes a [`Program`] (a flat list of
[`Instruction`] values) produced by `sql-codegen` and a [`Backend`] (the
storage layer from `sql-backend`), executes the program, and returns a
[`QueryResult`].

## Architecture

The VM is a **stack machine** with three workspaces:

| Workspace | Purpose |
|-----------|---------|
| Evaluation stack | `Vec<SqlValue>` — arithmetic, comparisons, expressions |
| Cursor map | Buffered table rows, keyed by alias |
| Output buffer | Rows assembled via `BeginRow` / `EmitColumn` / `EmitRow` |

### Execution phases

1. **Label scan** — O(n) pass to build a `{label → index}` map.
2. **Main loop** — fetch-decode-execute until `Halt`.
3. **Post-op pass** — process `SortResult`, `DistinctResult`, `LimitResult` that live after `Halt`.
4. **Materialize** — flatten the output buffer into a `QueryResult`.

### Cursor model

Cursors are **eagerly buffered** at `OpenScan` time.  This design choice:
- Satisfies the borrow checker (no dangling iterator reference into the backend).
- Allows nested-loop joins to re-open the inner cursor for each outer row.
- Makes `JumpIfExhausted` a simple boolean flag check.

The exhaustion model: `AdvanceCursor` fetches the current row and marks
`exhausted = false`.  If no row was available, it marks `exhausted = true`.
`JumpIfExhausted` reads the flag — not `pos >= len` — so the last valid row is
consumed before the jump fires.

### SQL three-valued logic

All comparisons propagate NULL.  `AND` and `OR` implement Kleene logic:

| A     | B     | A AND B | A OR B |
|-------|-------|---------|--------|
| false | NULL  | false   | NULL   |
| true  | NULL  | NULL    | true   |
| NULL  | NULL  | NULL    | NULL   |

### LIKE matching

LIKE is implemented as an **iterative NFA** (no regex, no recursion) using a
backtrack pointer.  This is O(n·m) worst-case with O(1) extra space, and
completely eliminates ReDoS risk.  Both `%` (zero-or-more) and `_` (exactly one)
wildcards are supported; matching is case-insensitive per the SQL standard.

### Aggregates

Six aggregate functions are supported: `COUNT(*)`, `COUNT(col)`, `SUM`,
`AVG`, `MIN`, `MAX`.  Each gets its own accumulator slot (allocated by
`InitAgg(n)`).  All functions skip NULL values except `COUNT(*)`.

### Post-processing operators

| Instruction | Effect |
|-------------|--------|
| `SortResult(keys)` | Stable-sort the output buffer by a key list |
| `DistinctResult` | Remove duplicate rows (preserves first occurrence) |
| `LimitResult(count, offset)` | Slice the buffer with OFFSET + LIMIT |

## Level 1 limitations

- `UpdateRows` and `DeleteRows` count affected rows but do not persistently
  modify the backend.  The `Backend` trait's `update()` / `delete()` methods
  require a `Cursor` with a specific `table_key()` that can only be constructed
  by the backend itself via `InMemoryBackend::open_cursor` (a non-trait method).
  A Level 2 VM will extend the trait or instruction set to close this gap.

## Usage

```rust
use coding_adventures_sql_backend::InMemoryBackend;
use coding_adventures_sql_codegen::{compile, Instruction, Program};
use coding_adventures_sql_vm::execute;

let program = Program {
    instructions: vec![
        Instruction::BeginRow,
        Instruction::LoadConst(coding_adventures_sql_backend::SqlValue::Int(42)),
        Instruction::EmitColumn("answer".to_string()),
        Instruction::EmitRow,
        Instruction::Halt,
    ],
};

let mut backend = InMemoryBackend::new();
let result = execute(&program, &mut backend).unwrap();
assert_eq!(result.columns, vec!["answer"]);
assert_eq!(result.rows[0][0], coding_adventures_sql_backend::SqlValue::Int(42));
```

## Test coverage

75 unit tests covering:
- Stack operations (LoadConst, EmitColumn, EmitRow)
- Binary operators including NULL propagation and Kleene AND/OR
- IS NULL / IS NOT NULL
- Unary operators (Neg, Not)
- LIKE pattern matching
- BETWEEN (inclusive/exclusive)
- IN list membership
- Table scans with filter loops
- All six aggregate functions
- SortResult, DistinctResult, LimitResult (and combinations)
- INSERT, CREATE TABLE, DROP TABLE (IF NOT EXISTS / IF EXISTS)
- Transactions (BEGIN / COMMIT / ROLLBACK)
- Error cases (StackUnderflow, DivisionByZero, LabelNotFound, AggIndexOutOfRange)
