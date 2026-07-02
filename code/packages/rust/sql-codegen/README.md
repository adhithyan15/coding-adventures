# coding-adventures-sql-codegen

Bytecode code generator for the Mini-SQLite SQL processing pipeline (Level 1).

## Pipeline position

```text
sql-lexer → sql-parser → sql-planner → sql-optimizer → sql-codegen → sql-vm → mini-sqlite
```

This crate is the **fifth stage**: it accepts an `OptimizedPlan` from
`sql-optimizer` and produces a `Program` — a flat list of `Instruction` values
that the `sql-vm` can execute.

## What it does

The code generator compiles an optimized query plan tree into a linear bytecode
program using a recursive tree walk:

1. **Peel post-ops** — Strip `Sort`, `Limit`, and `Distinct` wrappers from the
   outermost plan.  These operate on the entire result buffer and are emitted
   after `Halt`.

2. **Compile the inner plan** — Recursively compile each plan node into
   scan-loop patterns (`OpenScan` / `AdvanceCursor` / `JumpIfExhausted` /
   `CloseScan`), expression code, and row assembly (`BeginRow` / `EmitColumn` /
   `EmitRow`).

3. **Emit expressions** — SQL expressions compile to stack-machine sequences
   where each sub-expression pushes a value; operators pop operands and push
   results.

## Public API

```rust
use coding_adventures_sql_codegen::{compile, Program};
use coding_adventures_sql_optimizer::optimize;
use coding_adventures_sql_planner::LogicalPlan;

let plan = LogicalPlan::Scan { table: "users".into(), alias: None };
let opt = optimize(plan);
let program: Program = compile(&opt);
```

## Instruction set highlights

| Category | Instructions |
|---|---|
| Stack ops | `LoadConst`, `LoadColumn` |
| Arithmetic/logic | `BinaryOpInstr`, `UnaryOpInstr` |
| Null tests | `IsNull`, `IsNotNull` |
| Predicates | `Between`, `Like`, `InList` |
| Scan control | `OpenScan`, `AdvanceCursor`, `JumpIfExhausted`, `CloseScan` |
| Row assembly | `BeginRow`, `EmitColumn`, `EmitRow` |
| Aggregation | `InitAgg`, `UpdateAgg`, `FinalizeAgg`, `SaveGroupKey` |
| Control flow | `Label`, `Jump`, `JumpIfTrue`, `JumpIfFalse`, `Halt` |
| DDL | `CreateTableInstr`, `DropTableInstr` |
| DML | `InsertRow`, `UpdateRows`, `DeleteRows` |
| Transactions | `BeginTransaction`, `CommitTransaction`, `RollbackTransaction` |
| Post-processing | `SortResult`, `DistinctResult`, `LimitResult` |

## Security

A thread-local recursion depth counter (`MAX_DEPTH = 512`) prevents stack
overflow on pathologically deep expressions.

## Dependencies

- `coding-adventures-sql-optimizer` — provides `OptimizedPlan`
- `coding-adventures-sql-planner` — provides expression types (`SqlExpr`, `BinaryOp`, etc.)
- `coding-adventures-sql-backend` — provides `SqlValue` and `ColumnDef`

## Testing

```sh
cargo test --package coding-adventures-sql-codegen
```

The test suite contains 50+ unit tests covering all major compilation paths.
