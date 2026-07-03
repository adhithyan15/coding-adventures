# CodingAdventures.SqlCodegen

C# bytecode code generator for the Mini-SQLite Level 1 pipeline.

## What it does

`SqlCodegen` compiles an `OptimizedPlan` tree (produced by the C# `sql-optimizer`) into a
flat `Program` — a linear list of `Instruction` bytecodes that the SQL VM executes.

```
LogicalPlan  →  [sql-optimizer]  →  OptimizedPlan  →  [sql-codegen]  →  Program
```

## Where it fits

| Layer | Package | Output |
|-------|---------|--------|
| Parsing | `sql-lexer` + `sql-parser` | `Statement` AST |
| Planning | `sql-planner` | `LogicalPlan` tree |
| Optimization | `sql-optimizer` | `OptimizedPlan` tree |
| **Code generation** | **`sql-codegen`** | **`Program` (bytecode)** |
| Execution | `sql-vm` | result rows |

## Usage

```csharp
using CodingAdventures.SqlPlanner;
using CodingAdventures.SqlOptimizer;
using CodingAdventures.SqlCodegen;

// Given an OptimizedPlan from the optimizer:
OptimizedPlan opt = SqlOptimizer.Optimize(logicalPlan);

// Compile to bytecode:
Program prog = SqlCodegen.CompileOptimized(opt);

// Or, compile directly from a LogicalPlan (lifts + optimizes internally):
Program prog2 = SqlCodegen.Compile(logicalPlan);

// Inspect the result:
Console.WriteLine($"Instructions: {prog.Instructions.Count}");
Console.WriteLine($"Result schema: {string.Join(", ", prog.ResultSchema)}");
Console.WriteLine($"Labels: {prog.Labels.Count}");
```

## Compilation strategy

### Wrapper peeling

The outer `OptSort / OptLimit / OptDistinct` wrappers are stripped first and
collected as "post-op" instructions (`SortResult`, `LimitResult`, `DistinctResult`)
that are appended after the main scan body in canonical order:

```
Sort → Limit → Distinct → Halt
```

### Scan body

Data-producing nodes are compiled into nested scan loops:

```
OpenScan(cid, "users")
scan_0_loop:
  AdvanceCursor(cid, scan_0_end)
  ; body (filter, join, emit)
  Jump(scan_0_loop)
scan_0_end:
CloseScan(cid)
```

### Joins

- **INNER / CROSS JOIN**: nested loops, optional condition check with `JumpIfFalse`.
- **LEFT OUTER JOIN**: outer loop emits `JoinBeginRow`, inner loop sets `JoinSetMatched`
  on each hit, then `JoinIfMatched` skips the null-padded fallback emit.

### Aggregation

Two-phase:
1. **Accumulation scan** — for each row, `SaveGroupKey` + `InitAgg` + [arg expr] + `UpdateAgg`.
2. **Group emit loop** — `AdvanceGroupKey` + `LoadGroupKey` + `FinalizeAgg` + `EmitColumn` + `EmitRow`.

### DML / DDL

- **INSERT**: compile each row's value expressions then `InsertRow`.
- **UPDATE**: cursor-scan loop with optional predicate, then assignment expressions + `UpdateRows`.
- **DELETE**: cursor-scan loop with optional predicate, then `DeleteRows`.
- **CREATE TABLE / DROP TABLE**: single instruction, no loop.

## Instruction set overview

| Category | Instructions |
|----------|-------------|
| Stack | `LoadConst`, `LoadColumn`, `Pop` |
| Arithmetic / logic | `BinaryOpInstr`, `UnaryOpInstr` |
| Predicates | `IsNullInstr`, `IsNotNullInstr`, `BetweenInstr`, `InListInstr`, `LikeInstr` |
| Functions | `CallScalar` |
| Cursors | `OpenScan`, `AdvanceCursor`, `CloseScan` |
| Row construction | `BeginRow`, `EmitColumn`, `EmitRow`, `SetResultSchema` |
| Aggregation | `InitAgg`, `UpdateAgg`, `FinalizeAgg`, `SaveGroupKey`, `LoadGroupKey`, `AdvanceGroupKey` |
| Post-ops | `SortResult`, `LimitResult`, `DistinctResult` |
| Joins | `JoinBeginRow`, `JoinSetMatched`, `JoinIfMatched` |
| DML | `InsertRow`, `UpdateRows`, `DeleteRows` |
| DDL | `CreateTableInstr`, `DropTableInstr` |
| Control flow | `CodegenLabel`, `Jump`, `JumpIfFalse`, `JumpIfTrue`, `Halt` |

## Running tests

```bash
dotnet test tests/CodingAdventures.SqlCodegen.Tests/ --disable-build-servers
```

Coverage (≥80% line threshold enforced via coverlet):

```bash
dotnet test tests/CodingAdventures.SqlCodegen.Tests/ \
  --disable-build-servers \
  /p:CollectCoverage=true \
  "/p:Include=[CodingAdventures.SqlCodegen]*" \
  /p:Threshold=80 /p:ThresholdType=line
```

## Dependencies

- `CodingAdventures.SqlOptimizer` (direct)
- `CodingAdventures.SqlPlanner` (transitive via sql-optimizer)
- .NET 9.0 or later
