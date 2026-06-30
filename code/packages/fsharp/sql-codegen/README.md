# CodingAdventures.SqlCodegen.FSharp

F# bytecode code generator for the Mini-SQLite Level 1 pipeline.

## What it does

`sql-codegen` transforms an `OptimizedPlan` (from the optimizer) into a flat list of stack-machine bytecode instructions (`Program`). The generated `Program` is then executed by `sql-vm` to produce query results.

## Pipeline position

```
sql-lexer → sql-parser → sql-planner → sql-optimizer → [sql-codegen] → sql-vm → mini-sqlite
```

## Architecture

The codegen is a pure function over the plan tree — no I/O, no state, no database access.

### Stack machine model

The VM is a loop: fetch instruction, dispatch, advance. Values live on a single stack. Instructions pop inputs and push outputs. The expression `a + (b * 2)` compiles to:

```
LoadColumn(None, "a")    ← push a
LoadColumn(None, "b")    ← push b
LoadConst(Integer 2)     ← push 2
BinaryOpInstr(Mul)       ← pop 2 and b, push b*2
BinaryOpInstr(Add)       ← pop b*2 and a, push a+b*2
```

### Supported query types

| Query type | Key instructions emitted |
|-----------|--------------------------|
| SELECT (simple) | OpenScan, AdvanceCursor, BeginRow, EmitColumn, EmitRow, CloseScan |
| SELECT with WHERE | + JumpIfFalse (filter guard) |
| SELECT with GROUP BY | + InitAgg, UpdateAgg, SaveGroupKey, LoadGroupKey, FinalizeAgg |
| SELECT with ORDER BY | + SortResult (post-op) |
| SELECT DISTINCT | + DistinctResult (post-op) |
| SELECT LIMIT/OFFSET | + LimitResult (post-op) |
| SELECT with JOIN | Nested OpenScan loops |
| INSERT VALUES | LoadConst/LoadColumn, InsertRow |
| UPDATE | OpenScan loop + UpdateRows |
| DELETE | OpenScan loop + DeleteRows |
| CREATE TABLE | CreateTable |
| DROP TABLE | DropTable |
| Empty result | Halt only |

### Two-phase aggregation

Aggregates compile to a two-phase pattern:

1. **Accumulate** (inside the scan loop): `UpdateAgg` feeds each row's value to an accumulator
2. **Finalize** (after the loop): `FinalizeAgg` computes the final value (e.g. SUM÷COUNT for AVG)

## Usage

```fsharp
open CodingAdventures.SqlOptimizer.FSharp
open CodingAdventures.SqlCodegen.FSharp

let optimizedPlan = SqlOptimizer.optimize logicalPlan
let program = SqlCodegen.compile optimizedPlan
// program.Instructions is now a flat list ready for the VM
```

## API

```fsharp
module SqlCodegen =
    val compile : OptimizedPlan -> Program
    val compileExpression : Expr -> Instruction list
```

`compileExpression` is exported for testing and reuse; it compiles a single expression to a flat instruction sequence that leaves one value on the stack.

## Types

```fsharp
type Program = { Instructions: Instruction list }

[<RequireQualifiedAccess>]
type Instruction =
    // Stack
    | LoadConst | LoadColumn | LoadParam | LoadGroupKey | LoadOuterColumn | Pop
    // Arithmetic / comparison
    | BinaryOpInstr | UnaryOpInstr
    // Predicate tests
    | IsNull | IsNotNull | Between | Like | InList
    // Cursor control
    | OpenScan | AdvanceCursor | JumpIfExhausted | CloseScan
    // Row construction
    | BeginRow | EmitColumn | EmitRow
    // Aggregation
    | InitAgg | UpdateAgg | FinalizeAgg | SaveGroupKey | LoadGroupKey | AdvanceGroup
    // Control flow
    | Label | Jump | JumpIfTrue | JumpIfFalse | Halt
    // DDL
    | CreateTable | DropTable
    // DML
    | InsertRow | UpdateRows | DeleteRows
    // Transactions
    | BeginTransaction | CommitTransaction | RollbackTransaction
    // Post-processing
    | SortResult | DistinctResult | LimitResult
```
