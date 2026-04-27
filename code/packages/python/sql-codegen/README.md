# sql-codegen

Compiles optimized `LogicalPlan` trees from [`sql-planner`](../sql-planner) /
[`sql-optimizer`](../sql-optimizer) into a flat stream of IR bytecode for
[`sql-vm`](../sql-vm) to execute.

## Where it fits

```
┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌────────┐
│ sql-planner  │→ │ sql-optimizer│→ │ sql-codegen  │→ │ sql-vm │
│ LogicalPlan  │  │ LogicalPlan' │  │   Program    │  │ result │
└──────────────┘  └──────────────┘  └──────────────┘  └────────┘
```

Codegen is **pure** — no I/O, no backend dependency. Given a plan, it
produces a `Program { instructions, labels, result_schema }`.

## Why bytecode?

A `LogicalPlan` tree is great for reasoning and optimization but awkward
for direct execution — recursive interpreters are hard to debug, profile,
and port. A flat instruction stream turns the VM into a dispatch loop:
fetch, execute, advance. This is the same trick that Python's bytecode
compiler, the JVM, WebAssembly, and SQLite's VDBE all use.

## Usage

```python
from sql_planner import plan, SelectStmt
from sql_optimizer import optimize
from sql_codegen import compile

plan_tree = plan(select_stmt, schema_provider)
optimized = optimize(plan_tree)
program = compile(optimized)

for i, ins in enumerate(program.instructions):
    print(f"{i:02d}  {ins}")
```

## Instruction set summary

| Category     | Instructions                                               |
| ------------ | ---------------------------------------------------------- |
| Stack        | `LoadConst`, `LoadColumn`, `Pop`                           |
| Arithmetic   | `BinaryOp`, `UnaryOp`, `IsNull`, `IsNotNull`, `Between`, `InList`, `Like`, `Coalesce`, `CallScalar` |
| Scan         | `OpenScan`, `AdvanceCursor`, `CloseScan`                   |
| Output       | `BeginRow`, `EmitColumn`, `EmitRow`, `SetResultSchema`, `ScanAllColumns` |
| Aggregate    | `InitAgg`, `UpdateAgg`, `FinalizeAgg`, `SaveGroupKey`, `LoadGroupKey`, `AdvanceGroupKey` |
| Post-proc    | `SortResult`, `LimitResult`, `DistinctResult`              |
| Set ops      | `CaptureLeftResult`, `IntersectResult`, `ExceptResult`     |
| Mutation     | `InsertRow`, `InsertFromResult`, `UpdateRows`, `DeleteRows`, `CreateTable`, `DropTable` |
| Transactions | `BeginTransaction`, `CommitTransaction`, `RollbackTransaction` |
| Control      | `Label`, `Jump`, `JumpIfFalse`, `JumpIfTrue`, `Halt`       |

See [`../../../specs/sql-codegen.md`](../../../specs/sql-codegen.md) for full
instruction semantics.
