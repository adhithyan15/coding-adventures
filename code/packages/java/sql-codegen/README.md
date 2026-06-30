# coding-adventures-sql-codegen

Java bytecode compiler for the SQL VM. Transforms an `OptimizedPlan` (from
`coding-adventures-sql-optimizer`) into a flat `Program` consisting of typed
bytecode instructions, a label-to-index jump table, and an output column schema.

## Where it fits

```
sql-parser (stub)
    └─▶ sql-planner   (LogicalPlan)
            └─▶ sql-optimizer   (OptimizedPlan)
                    └─▶ sql-codegen   (Program)  ◀── this package
                                └─▶ sql-vm   (executes Program)
```

## Usage

```java
import com.codingadventures.sqlcodegen.SqlCodegen;
import com.codingadventures.sqlcodegen.SqlCodegen.Program;
import com.codingadventures.sqlplanner.SqlPlanner;
import com.codingadventures.sqloptimizer.SqlOptimizer;

// Compile from a LogicalPlan (optimizer is applied automatically)
Program prog = SqlCodegen.compile(logicalPlan);

// Compile from an already-optimised plan
SqlOptimizer.OptimizedPlan opt = SqlOptimizer.optimize(logicalPlan);
Program prog2 = SqlCodegen.compileOptimized(opt);

// Inspect the compiled program
System.out.println("Instructions: " + prog.instructions().size());
System.out.println("Schema:        " + prog.resultSchema());
prog.instructions().forEach(System.out::println);
```

## Program structure

A `Program` is a triple:

| Field | Type | Description |
|---|---|---|
| `instructions` | `List<Instruction>` | Flat ordered bytecode list |
| `labels` | `Map<String, Integer>` | Label name → instruction index |
| `resultSchema` | `List<String>` | Output column names in order |

## Instruction set overview

### Stack operations
| Instruction | Effect |
|---|---|
| `LoadConst(value)` | Push a compile-time constant |
| `LoadColumn(cursorId, column)` | Push a column value from cursor |
| `Pop()` | Discard stack top |
| `BinaryOp(op)` | Pop 2, push result |
| `UnaryOp(op)` | Pop 1, push result |
| `IsNull()` / `IsNotNull()` | Pop 1, push boolean |
| `Between()` | Pop 3 (value, low, high), push boolean |
| `InList(n)` | Pop n items then needle, push boolean |
| `Like(negated)` | Pop value and pattern, push boolean |
| `CallScalar(func, nArgs)` | Pop nArgs, call scalar func, push result |

### Cursor operations
| Instruction | Effect |
|---|---|
| `OpenScan(cursorId, table)` | Open a full table scan |
| `AdvanceCursor(cursorId, onExhausted)` | Move to next row; jump if done |
| `CloseScan(cursorId)` | Release the cursor |

### Row building
| Instruction | Effect |
|---|---|
| `SetResultSchema(columns)` | Declare output column names (emitted once) |
| `BeginRow()` | Start assembling an output row |
| `EmitColumn(name)` | Pop and store as named column |
| `EmitRow()` | Finalise and emit the assembled row |

### Aggregate operations
| Instruction | Effect |
|---|---|
| `InitAgg(slot, func, distinct)` | Reset accumulator slot |
| `UpdateAgg(slot)` | Feed top-of-stack into accumulator |
| `FinalizeAgg(slot, func)` | Push final aggregate value |
| `SaveGroupKey(n)` | Pop n values as current group key |
| `LoadGroupKey(i)` | Push i-th group key value |
| `AdvanceGroupKey(label, hasGroupBy)` | Advance to next group; jump when done |

### Post-processing (applied after scan loop)
| Instruction | Effect |
|---|---|
| `SortResult(keys)` | Sort buffered result rows |
| `LimitResult(count, offset)` | Truncate result rows |
| `DistinctResult()` | Remove duplicate rows |

### LEFT JOIN tracking
| Instruction | Effect |
|---|---|
| `JoinBeginRow()` | Clear matched flag for current left row |
| `JoinSetMatched()` | Mark that a matching right row was found |
| `JoinIfMatched(label)` | Jump past null-padding if match was found |

### DML / DDL
| Instruction | Effect |
|---|---|
| `InsertRow(table, columns)` | Pop values and insert a row |
| `UpdateRows(table, cols, cursorId)` | Update current cursor row |
| `DeleteRows(table, cursorId)` | Delete current cursor row |
| `CreateTable(table, ifNotExists, cols)` | DDL: create table |
| `DropTable(table, ifExists)` | DDL: drop table |

### Control flow
| Instruction | Effect |
|---|---|
| `Label(name)` | Define a jump target |
| `Jump(target)` | Unconditional jump |
| `JumpIfFalse(target)` | Pop; jump if falsy |
| `JumpIfTrue(target)` | Pop; jump if truthy |
| `Halt()` | Terminate the program |

## Code generation patterns

### Simple SELECT

```
SetResultSchema(["name"])
OpenScan(0, "users")
Label("scan_0_loop_0")
AdvanceCursor(0, "scan_0_end_1")
  BeginRow()
  LoadColumn(0, "name")
  EmitColumn("name")
  EmitRow()
Jump("scan_0_loop_0")
Label("scan_0_end_1")
CloseScan(0)
Halt()
```

### SELECT with WHERE

```
...scan loop...
  [predicate expression]
  JumpIfFalse("filter_skip_2")
  [row building]
  Label("filter_skip_2")
...
```

### SELECT with ORDER BY + LIMIT

Post-processing instructions appear after the scan loop and before Halt:
```
...scan loop + row building...
SortResult([SortKey("name", ASC, LAST)])
LimitResult(10, 0)
Halt()
```

### COUNT(*) aggregate

```
OpenScan(0, "users")
Label("scan_0_loop_0")
AdvanceCursor(0, "scan_0_end_1")
  SaveGroupKey(0)
  InitAgg(0, COUNT_STAR, false)
  LoadConst(null)
  UpdateAgg(0)
Jump("scan_0_loop_0")
Label("scan_0_end_1")
CloseScan(0)
Label("group_start_2")
AdvanceGroupKey("group_end_3", false)
BeginRow()
FinalizeAgg(0, COUNT_STAR)
EmitColumn("cnt")
EmitRow()
Jump("group_start_2")
Label("group_end_3")
Halt()
```

## Dependencies

- `coding-adventures-sql-planner` — `LogicalPlan`, `SqlExpr`, `OutputColumn`, etc.
- `coding-adventures-sql-optimizer` — `OptimizedPlan`, `SqlOptimizer.optimize()`

## Building

```bash
# Build dependency JARs first
(cd ../sql-planner  && gradle jar --quiet)
(cd ../sql-optimizer && gradle jar --quiet)

# Run tests with coverage verification
gradle test
```

Coverage is enforced at ≥ 80% line coverage via JaCoCo.
