# sql-vm (Kotlin)

The Kotlin stack-machine VM that executes bytecode `Program` objects produced by `sql-codegen`.

## What is this?

`sql-vm` is the execution engine of the mini-sqlite Level 1 pipeline:

```
SQL text
  │  sql-lexer / sql-parser
  ▼
AST
  │  sql-planner
  ▼
LogicalPlan
  │  sql-optimizer
  ▼
OptimizedPlan
  │  sql-codegen
  ▼
Program (flat instruction list)
  │  sql-vm  ← THIS PACKAGE
  ▼
QueryResult
```

## How does it work?

The VM is a simple dispatch loop — a `while (pc < instructions.size)` that reads one `Instruction` at a time, executes it, and advances the program counter. Control-flow instructions (`Jump`, `JumpIfFalse`, `JumpIfTrue`) rewrite the program counter directly.

### Value representation

All SQL values flow through the `SqlValue` sealed class from `sql-codegen`:

| Kotlin type          | SQL type       |
|----------------------|----------------|
| `SqlValue.Null`      | NULL           |
| `SqlValue.IntVal`    | INTEGER (Long) |
| `SqlValue.FloatVal`  | REAL (Double)  |
| `SqlValue.TextVal`   | TEXT           |
| `SqlValue.BoolVal`   | BOOLEAN        |

### Stack machine

The VM is register-free. All intermediate values live on an `ArrayDeque<SqlValue>` stack:
- `push(v)` → `stack.addLast(v)`
- `pop()` → `stack.removeLast()`

### Cursors

Table scans are backed by the `Backend` interface from `sql-backend`. Each `OpenScan` instruction opens a cursor; `AdvanceCursor` moves it to the next row (or jumps to an end label if exhausted); `CloseScan` releases the cursor.

### Aggregation

Two-phase:
1. **Accumulate** (inside the scan loop): `InitAgg` / `UpdateAgg` feed each row's value into a running accumulator keyed by the current GROUP BY key.
2. **Finalize** (after the scan): `AdvanceGroup` / `FinalizeAgg` iterate over accumulated groups and emit one result row per group.

## Public API

```kotlin
object SqlVm {
    fun execute(program: Program, backend: Backend): QueryResult
}

data class QueryResult(
    val columns: List<String>,
    val rows: List<List<SqlValue>>,
    val rowsAffected: Int,
)
```

## Usage

```kotlin
val backend = InMemoryBackend()
val program: Program = SqlCodegen.compile(SqlOptimizer.optimize(SqlPlanner.plan(ast)))
val result: QueryResult = SqlVm.execute(program, backend)

println(result.columns)        // ["id", "name"]
result.rows.forEach { println(it) }
```

## Three-valued logic

SQL uses three-valued logic for NULL:

| Expression     | Result |
|----------------|--------|
| `NULL AND TRUE`  | NULL   |
| `NULL AND FALSE` | FALSE  |
| `NULL OR TRUE`   | TRUE   |
| `NULL OR FALSE`  | NULL   |
| `NULL = anything`| NULL   |
| `NULL IS NULL`   | TRUE   |

The VM implements these semantics in `evalBinary` / `evalAnd` / `evalOr`.

## Package stack

| Package        | Role                          |
|----------------|-------------------------------|
| `sql-backend`  | Storage interface + in-memory implementation |
| `sql-planner`  | AST → logical plan            |
| `sql-optimizer`| Logical plan → optimized plan |
| `sql-codegen`  | Optimized plan → bytecode     |
| **`sql-vm`**   | **Bytecode → QueryResult**    |

## Building

```bash
# Pre-build all dependencies (leaf-to-root order):
(cd ../sql-backend  && gradle jar --quiet)
(cd ../sql-planner  && gradle jar --quiet)
(cd ../sql-optimizer && gradle jar --quiet)
(cd ../sql-codegen  && gradle jar --quiet)

# Build and test:
gradle test jacocoTestReport jacocoTestCoverageVerification
```

The build output goes to `gradle-build/` (NOT `build/`) to avoid a
case-insensitive filesystem collision with the `BUILD` script file on macOS and Windows.
