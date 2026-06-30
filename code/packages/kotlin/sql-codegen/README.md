# sql-codegen (Kotlin)

Kotlin bytecode code generator for the mini-sqlite Level 1 pipeline.

## What it does

`sql-codegen` takes an `OptimizedPlan` produced by `sql-optimizer` and compiles
it into a flat list of stack-machine bytecode instructions that the `sql-vm` can
execute in a simple loop.

```
OptimizedPlan (sql-optimizer)
    │  SqlCodegen.compile()
    ▼
Program { instructions: List<Instruction> }
    │  sql-vm (next stage)
    ▼
QueryResult
```

## Why compile to bytecode?

A plan tree is great for optimisation but awkward for direct execution — tree
traversal requires recursion, and recursive interpreters are hard to debug, hard
to profile, and hard to port.

A *flat* bytecode program solves this: the VM is a simple loop — read the next
instruction, execute it, advance the program counter.  No recursion.  Each
instruction is small and self-contained.  This is the same insight behind
CPython, the JVM, WebAssembly, and SQLite's VDBE.

## Where it fits

```
Depends on: sql-planner (for SqlExpr / ColumnDef / SortKey types)
            sql-optimizer (for OptimizedPlan)
Used by:    sql-vm (executes the Program)
```

## Package

```kotlin
package com.codingadventures.sqlcodegen
```

## Usage

```kotlin
import com.codingadventures.sqloptimizer.SqlOptimizer
import com.codingadventures.sqlcodegen.SqlCodegen

val optimized = SqlOptimizer.optimize(logicalPlan)
val program   = SqlCodegen.compile(optimized)

for (instr in program.instructions) {
    println(instr)
}
```

## Instruction set summary

| Category    | Instructions |
|-------------|--------------|
| Stack       | LoadConst, LoadColumn, LoadParam, LoadGroupKey, Pop |
| Arithmetic  | BinaryOpInstr, UnaryOpInstr |
| Predicates  | IsNull, IsNotNull, Between, Like, InList |
| Scan        | OpenScan, AdvanceCursor, JumpIfExhausted, CloseScan |
| Row output  | BeginRow, EmitColumn, EmitRow |
| Aggregation | InitAgg, UpdateAgg, FinalizeAgg, SaveGroupKey, AdvanceGroup |
| Control     | Label, Jump, JumpIfTrue, JumpIfFalse, Halt |
| DDL         | CreateTableInstr, DropTableInstr |
| DML         | InsertRow, UpdateRows, DeleteRows |
| Transactions| BeginTransaction, CommitTransaction, RollbackTransaction |
| Post-ops    | SortResult, DistinctResult, LimitResult |

## Running tests

```bash
# From the package directory — pre-builds sibling JARs first
./BUILD   # Linux / macOS
BUILD_windows  # Windows
```

Or directly with Gradle:

```bash
(cd ../sql-planner && gradle jar --quiet)
(cd ../sql-optimizer && gradle jar --quiet)
gradle test jacocoTestReport jacocoTestCoverageVerification
```

## Test coverage

The test suite targets ≥80% line coverage (enforced by JaCoCo) and includes:

- All 16 plan-node types
- All 13 BinaryOp variants
- Both UnaryOp variants (NEG, NOT)
- IsNull, IsNotNull, Between, Like, InList expression tests
- All 5 AggFn variants (COUNT, COUNT_STAR, SUM, AVG, MIN, MAX)
- Post-op peeling (Sort, Limit, Distinct wrappers)
- Multi-row INSERT
- Label naming convention tests
- Jump label consistency tests (every Jump target resolves)
- SqlValue and Instruction structural tests
