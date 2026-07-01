# CodingAdventures.SqlVm

A C# stack-machine virtual machine that executes bytecode `Program` objects
produced by [`CodingAdventures.SqlCodegen`](../sql-codegen/). It forms the
execution layer of the Mini-SQLite Level 1 pipeline.

## Position in the stack

```
SQL text
  ↓  CodingAdventures.SqlLexer / SqlParser
AST
  ↓  CodingAdventures.SqlPlanner
LogicalPlan
  ↓  CodingAdventures.SqlOptimizer
OptimizedPlan
  ↓  CodingAdventures.SqlCodegen
Program  ←── you are here
  ↓  CodingAdventures.SqlVm
QueryResult
```

## Usage

```csharp
using CodingAdventures.SqlBackend;
using CodingAdventures.SqlCodegen;
using CodingAdventures.SqlVm;

// Build a program (via codegen) and execute it.
Program program = SqlCodegen.Compile(logicalPlan);
Backend backend = new InMemoryBackend();

QueryResult result = SqlVm.Execute(program, backend);

// SELECT results
foreach (var row in result.Rows)
    Console.WriteLine(string.Join(", ", row));

// DML count
Console.WriteLine($"Rows affected: {result.RowsAffected}");
```

## Design

### Execution model

The VM is a simple fetch-decode-execute loop:

1. **Pre-scan** the instruction list to build a `label → index` dictionary
   for O(1) jump resolution.
2. **Main loop**: execute one instruction per iteration, maintaining a value
   stack (`Stack<object?>`), open cursor dictionary, aggregate accumulator
   table, and output row buffer.
3. **Post-processing**: after the loop, apply sort / distinct / limit in order.
4. **Return** a `QueryResult` record.

### Value types

SQL values are CLR `object?` values:

| SQL type | CLR type   |
|----------|-----------|
| NULL     | `null`    |
| INTEGER  | `long`    |
| REAL     | `double`  |
| TEXT     | `string`  |
| BLOB     | `byte[]`  |
| BOOLEAN  | `bool`    |

### NULL semantics

Three-valued logic (3VL) is implemented throughout:

- Any arithmetic with NULL yields NULL.
- `NULL AND FALSE` → `FALSE` (short-circuit).
- `NULL OR TRUE` → `TRUE` (short-circuit).
- `NULL AND TRUE` / `NULL OR FALSE` → `NULL`.
- `IS NULL` / `IS NOT NULL` always yield a non-null bool.
- Jumps treat NULL as falsy (same as `FALSE`).

### Aggregate functions

Two-phase accumulation:

1. **Scan phase**: `InitAgg` + `UpdateAgg` per input row, keyed by group key.
2. **Emit phase**: `AdvanceGroupKey` iterates over collected groups;
   `FinalizeAgg` converts each accumulator into its final value.

Empty-table handling: if no rows were scanned and no GROUP BY is present,
the VM synthesises one empty group so that `COUNT(*)` returns 0 and
`AVG(x)` returns NULL, matching SQL standard behaviour.

### Post-processing

`SortResult`, `LimitResult`, and `DistinctResult` are emitted by the codegen
as the *last* instructions before `Halt`. The VM records their parameters
during the main loop and applies them to the completed `OutputRows` list in
order: sort → limit/offset → distinct.

## Public API

```csharp
// Execute a compiled program.
QueryResult SqlVm.Execute(Program program, Backend backend);

// Result type.
record QueryResult(
    IReadOnlyList<string> Columns,
    IReadOnlyList<IReadOnlyList<object?>> Rows,
    int RowsAffected);

// Utility: test SQL LIKE pattern matching directly.
bool SqlVm.LikeMatch(string value, string pattern);
```

## Dependencies

- `CodingAdventures.SqlBackend` — `Backend`, `InMemoryBackend`, `Row`, `ICursor`, value types.
- `CodingAdventures.SqlCodegen` — `Program`, `Instruction` hierarchy, enums.
- `CodingAdventures.SqlPlanner` (transitive via SqlCodegen) — `ColumnDef`, etc.
- `CodingAdventures.SqlOptimizer` (transitive via SqlCodegen).

## Testing

```sh
dotnet test tests/CodingAdventures.SqlVm.Tests/CodingAdventures.SqlVm.Tests.csproj
```

The test suite has 100+ tests covering: SELECT, WHERE, aggregates, ORDER BY,
LIMIT, DISTINCT, INSERT, UPDATE, DELETE, CREATE/DROP TABLE, NULL semantics,
BETWEEN, IN, LEFT JOIN, scalar functions, LIKE matching, and error paths.
