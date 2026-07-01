# sql-vm (Haskell)

Stack-machine bytecode interpreter for the Mini-SQLite Level 1 pipeline.

## What It Does

`sql-vm` executes a `Program` (produced by `sql-codegen`) against an
`InMemoryBackend` (from `sql-backend`) and returns a `QueryResult` containing
column names, data rows, and a rows-affected count for DML statements.

## Where It Fits

```
sql-lexer → sql-parser → sql-planner → sql-optimizer → sql-codegen → [sql-vm]
```

`sql-vm` is the final execution stage.  Every upstream stage (lexing, parsing,
planning, optimising, code generation) produces artefacts that flow through the
pipeline; `sql-vm` evaluates those artefacts against actual data.

## Architecture

### Stack machine

The VM maintains a single value stack.  Each `Instruction` pops zero, one, or
two values from the top of the stack, computes a result, and pushes it back.

```
LoadConst (LitInt 3)   -- stack: [3]
LoadConst (LitInt 4)   -- stack: [3, 4]
BinaryOpInstr Add      -- stack: [7]
```

This design is identical to SQLite's VDBE, the JVM, and CPython — the simplest
possible interpreter architecture.

### StateT monad

The VM runs in `StateT VmState IO`.  Pure dispatch uses `get`/`put`/`modify`;
backend calls (which are `Either`-valued pure functions) are applied inside the
`StateT` via `liftIO`.  The mutable `InMemoryBackend` is wrapped in an `IORef`
so that backend operations that return a new backend value can be written back
without threading the value through every call site.

### Three-valued SQL logic

SQL has three truth values: `TRUE`, `FALSE`, and `NULL` (unknown).

| Expression          | Result |
|---------------------|--------|
| `NULL = 1`          | NULL   |
| `FALSE AND NULL`    | FALSE  |
| `TRUE OR NULL`      | TRUE   |
| `NOT NULL`          | NULL   |

All comparison and arithmetic operators propagate `NULL` unless overridden by
the short-circuit rules for `AND` and `OR`.

### Cursor peek pattern

`sql-codegen` generates scan loops in this form:

```
OpenScan "t" alias
Label "loop"
JumpIfExhausted alias "end"   ← test; jump if done
AdvanceCursor alias            ← move to next row
<body>
Jump "loop"
Label "end"
CloseScan alias
```

`JumpIfExhausted` peeks at the next row by calling `iteratorNext`.  If a row
exists, it is stored as the current row for that cursor *before* jumping (or
not jumping).  `AdvanceCursor` clears the peek flag so the next
`JumpIfExhausted` call advances the iterator again.  This avoids double-
advancing.

### Label pre-indexing

Before execution begins, `buildLabelIndex` walks the instruction list once and
maps every `Label` to its instruction index.  Jump instructions resolve labels
in O(1) via `Map.lookup`.

### Post-processing

`SortResult`, `DistinctResult`, and `LimitResult` record directives that are
applied *after* the scan loop fills the output buffer.  The order is always:

1. Sort (stable, multi-key, using `compareSqlValues` for type coercion)
2. Distinct (via `Data.List.nub` on value lists)
3. Limit / Offset (via `drop` + `take`)

This matches the SQL standard and the Python and F# reference implementations.

### LIKE matching

`LIKE` pattern matching uses a direct two-pointer recursive algorithm rather
than a compiled `Regex`, avoiding ReDoS vulnerabilities.  The matcher is
case-insensitive.  Wildcards:

- `%` — zero or more characters of any kind
- `_` — exactly one character

## Usage

```haskell
import SqlVm (execute, QueryResult(..))
import SqlBackend (newBackend, createTable, defaultColumnDef)
import SqlCodegen (Program(..), Instruction(..))

let be = ... -- build an InMemoryBackend
    prog = Program [...]
result <- execute prog be
print (rows result)
```

## Test Coverage

Run the test suite with:

```
cabal test all --test-show-details=direct
```

The test suite (`test/SqlVmSpec.hs`) contains 50+ hspec tests covering:

- Full table scan (SELECT *)
- Filtered scan (WHERE predicate)
- Aggregate functions: COUNT(*), COUNT, SUM, AVG, MIN, MAX
- Empty-table aggregates (COUNT=0, SUM/AVG/MIN=NULL)
- NULL propagation through arithmetic and comparison
- IS NULL / IS NOT NULL
- LIKE matching (%, _, case-insensitivity, NULL input)
- BETWEEN (inclusive, NULL propagation)
- IN list (hit, miss, NULL, empty list)
- SortResult (ascending, descending)
- LimitResult (limit, offset, both)
- DistinctResult
- InsertRow (rows affected, DML)
- CreateTableInstr (DDL)
- Multiple column emission
- Arithmetic operators (+, -, *, /, %)
- Unary negation (integer, real)
- String concatenation (||)
- LoadConst / LoadNull
- Halt terminates execution
- Comparison operators (<, <=, >, >=, <>)

## Dependencies

| Package         | Provides                                  |
|-----------------|-------------------------------------------|
| `sql-backend`   | `InMemoryBackend`, `SqlValue`, `Row`, iterators |
| `sql-codegen`   | `Program`, `Instruction`, `BinaryOp`, `AggFn` |
| `sql-planner`   | `LiteralVal`, `SortKey`, `SqlExpr`, `SortDir` |
| `containers`    | `Data.Map.Strict`                         |
| `mtl`           | `Control.Monad.State.Strict`              |
