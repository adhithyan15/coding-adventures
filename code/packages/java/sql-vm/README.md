# java/sql-vm

A Java 21 stack-machine virtual machine that executes bytecode `Program`s
produced by `java/sql-codegen`.

## Overview

`SqlVm` is the execution engine in the mini-sqlite pipeline:

```
SQL text
  │
  ▼
SqlPlanner  (parse + logical plan)
  │
  ▼
SqlOptimizer  (predicate push-down, etc.)
  │
  ▼
SqlCodegen  (bytecode Program)
  │
  ▼
SqlVm.execute(program, backend)  ← this package
  │
  ▼
QueryResult
```

The VM is a single-pass dispatch loop over a flat list of typed `Instruction`
records.  Each instruction is dispatched via Java 21 sealed-interface pattern
matching, producing readable one-liner handlers.

## Public API

```java
// Execute a compiled program against any Backend implementation.
QueryResult result = SqlVm.execute(program, backend);

// Result shape:
List<String>       result.columns();      // output column names
List<List<Object>> result.rows();         // result rows (null = SQL NULL)
int                result.rowsAffected(); // 0 for SELECT; N for DML
```

## Instruction set

The VM interprets all instructions produced by `SqlCodegen`:

| Category      | Instructions |
|---------------|-------------|
| Stack         | `LoadConst`, `LoadColumn`, `Pop` |
| Arithmetic    | `BinaryOp`, `UnaryOp`, `IsNull`, `IsNotNull` |
| Predicates    | `Between`, `InList`, `Like`, `CallScalar` |
| Scan          | `OpenScan`, `AdvanceCursor`, `CloseScan` |
| Row output    | `BeginRow`, `EmitColumn`, `EmitRow`, `SetResultSchema` |
| Aggregates    | `InitAgg`, `UpdateAgg`, `FinalizeAgg` |
| Groups        | `SaveGroupKey`, `LoadGroupKey`, `AdvanceGroupKey` |
| Post-ops      | `SortResult`, `LimitResult`, `DistinctResult` |
| JOIN tracking | `JoinBeginRow`, `JoinSetMatched`, `JoinIfMatched` |
| DML           | `InsertRow`, `UpdateRows`, `DeleteRows` |
| DDL           | `CreateTable`, `DropTable` |
| Control flow  | `Label`, `Jump`, `JumpIfFalse`, `JumpIfTrue`, `Halt` |

## Three-valued logic

SQL NULL propagates through all arithmetic and comparison operators:

- `NULL + anything → NULL`
- `NULL AND FALSE → FALSE` (short-circuit)
- `NULL OR  TRUE  → TRUE`  (short-circuit)
- `NULL AND TRUE  → NULL`
- `IS NULL(NULL)  → true`

## Aggregates

| Function   | NULL inputs | Empty group |
|------------|-------------|-------------|
| COUNT(*)   | counted     | 0           |
| COUNT(col) | skipped     | 0           |
| SUM        | skipped     | NULL        |
| AVG        | skipped     | NULL        |
| MIN / MAX  | skipped     | NULL        |

## LIKE matching

`%` matches any sequence of zero or more characters; `_` matches exactly one
character.  All Java regex metacharacters in the pattern are escaped before
compiling, so `LIKE '3.14'` matches only the literal string `3.14`.

## Stack layout conventions

```
-- BinaryOp: left was pushed first, right last (right is on top)
LoadConst left
LoadConst right
BinaryOp ADD    → pops right, pops left, pushes left+right

-- InsertRow: col0 pushed first, col(n-1) pushed last
LoadConst val0
LoadConst val1
InsertRow "t" ["col0", "col1"]

-- Between: value pushed first, then low, then high
LoadConst value
LoadConst low
LoadConst high
Between         → pops high, low, value; pushes boolean
```

## Dependencies

- `java/sql-backend` — `Backend`, `InMemoryBackend`, `Row`, `Cursor`
- `java/sql-planner` — `SqlPlanner.ColumnDef` (used in `CreateTable`)
- `java/sql-optimizer` — transitive (via sql-codegen)
- `java/sql-codegen` — `Program`, `Instruction` hierarchy, `AggFunc`, …

## Build

```sh
# Build all prerequisites first, then test:
(cd ../sql-backend && gradle jar --quiet) || true
(cd ../sql-planner  && gradle jar --quiet)
(cd ../sql-optimizer && gradle jar --quiet)
(cd ../sql-codegen  && gradle jar --quiet)
gradle test jacocoTestReport jacocoTestCoverageVerification
```

## Test coverage

55 JUnit 5 tests covering SELECT, WHERE, aggregates (COUNT/SUM/AVG/MIN/MAX),
ORDER BY, LIMIT/OFFSET, DISTINCT, INSERT, UPDATE, DELETE, CREATE/DROP TABLE,
NULL semantics, three-valued logic, LIKE, BETWEEN, IN list, LEFT JOIN tracking,
scalar functions, and empty-table aggregate edge cases.

JaCoCo coverage gate: ≥ 80% line coverage.
