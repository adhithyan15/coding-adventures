# coding-adventures-sql-codegen

Haskell bytecode code generator for the Mini-SQLite Level 1 pipeline.

## What It Does

`sql-codegen` transforms an `OptimizedPlan` (produced by `sql-optimizer`) into a
flat list of stack-machine `Instruction` values (`Program`) that the `sql-vm` can execute.

## Pipeline Position

```
sql-lexer → sql-parser → sql-planner → sql-optimizer → [sql-codegen] → sql-vm → mini-sqlite

Input : OptimizedPlan
Output: Program (a flat list of Instruction values)
```

## What Is a Stack Machine?

A stack machine is the simplest possible virtual computer. It has no named
registers — just a single stack of values and a sequence of instructions.
Each instruction pops zero, one, or two values from the top of the stack,
does some work, and pushes a result back.

SQLite's own query engine (VDBE), the JVM, and CPython all use stack machines
for the same reason: they're easy to generate code for, easy to execute in a
tight interpreter loop, and require no register allocation.

## Usage

```haskell
import SqlOptimizer (optimize)
import SqlPlanner   (plan, inMemorySchema, ...)
import SqlCodegen   (compile, instructions)

let schema  = inMemorySchema [("users", ["id", "name", "age"])]
let lp      = plan schema stmt
let op      = fmap optimize lp
let program = fmap compile op
-- program.instructions is ready for the VM
```

## Key Types

```haskell
-- Compiled output
newtype Program = Program { instructions :: [Instruction] }

-- Instruction set (partial)
data Instruction
  = LoadConst LiteralVal
  | LoadColumn (Maybe String) String
  | BinaryOpInstr BinaryOp
  | OpenScan String (Maybe String)
  | AdvanceCursor (Maybe String)
  | JumpIfExhausted (Maybe String) String
  | BeginRow | EmitColumn String | EmitRow
  | InitAgg Int | UpdateAgg Int AggFn | FinalizeAgg Int AggFn
  | SortResult [SortKey] | DistinctResult | LimitResult ...
  | Halt
  | ...
```

## Public API

| Function | Description |
|---|---|
| `compile :: OptimizedPlan -> Program` | Top-level entry point |
| `compileExpr :: SqlExpr -> [Instruction]` | Compile one expression (exported for testing) |

## Tests

```
cabal test all --test-show-details=direct
```

46+ hspec tests covering all plan node types, expression forms, and bytecode
structure invariants.

## Design Notes

- **Purely functional**: label counter is threaded as a plain `Int` through
  all compilation functions — no `IORef` or `State` monad needed.
- **Literate**: all code contains Knuth-style inline explanations, truth
  tables, and stack-state diagrams.
- **Post-op model**: Sort/Limit/Distinct wrappers are peeled off first and
  emitted after the scan loop, matching SQLite's VDBE approach.
- **Two-phase aggregation**: `InitAgg` before the loop, `UpdateAgg` inside,
  `FinalizeAgg` + `EmitRow` after.
