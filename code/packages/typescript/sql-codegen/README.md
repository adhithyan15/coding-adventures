# sql-codegen (TypeScript)

SQL code generator: compiles an optimized `LogicalPlan` (from
`@coding-adventures/sql-optimizer`) into a flat stack-machine IR `Program`
that `@coding-adventures/sql-vm` executes.

## Where It Fits

```
sql-parser → sql-planner → sql-optimizer → sql-codegen → sql-vm
                                                ↑
                                           this package
```

## Usage

```typescript
import { compile, CodegenError } from "@coding-adventures/sql-codegen";
import type { Program }           from "@coding-adventures/sql-codegen";

// `plan` is a LogicalPlan from sql-planner / sql-optimizer
const program: Program = compile(plan);
// program.instructions — flat bytecode array
// program.labels       — Map<label, instruction index>
// program.resultSchema — column names in output order
```

## Architecture

The code generator performs a single tree-walk over the `LogicalPlan` and
emits IR instructions into a flat array.

### Stack machine model

Every expression pushes one value onto the operand stack.
Every operator (comparison, arithmetic, function call) pops its operands and
pushes the result. A `BeginRow` clears the row buffer; `EmitColumn` stores the
top-of-stack into a named slot; `EmitRow` flushes the row buffer to the output.

### Aggregate two-phase approach

Aggregates (COUNT, SUM, MIN, MAX, AVG, GROUP_CONCAT) are compiled in two
phases:

1. **Scan phase** — `InitAgg` allocates slots; the scan loop runs
   `UpdateAgg` for each input row, optionally preceded by `SaveGroupKey` for
   GROUP BY.
2. **Emit phase** — after the scan, a group-iteration loop calls `FinalizeAgg`
   (reads the accumulated slot) and `LoadGroupKey` (reads the GROUP BY value),
   then emits one output row per group.

### ORDER BY on non-projected columns

When an ORDER BY key is not in the SELECT list, the codegen adds a hidden
`__sort_<expr>` column. The `SortResult` instruction sorts by that column,
then the `stripPrefix: "__sort_"` option removes it from the final output.

## IR Instruction Reference

See `src/ir.ts` for the full discriminated union. The `sql-vm.md` spec has
prose descriptions of each opcode's semantics.

## Supported LogicalPlan nodes

`ScanNode`, `FilterNode`, `ProjectNode`, `SortNode`, `LimitNode`,
`DistinctNode`, `AggregateNode`, `HavingNode`, `JoinNode`,
`InsertNode`, `UpdateNode`, `DeleteNode`,
`CreateTableNode`, `DropTableNode`, `TransactionNode`

## Test coverage

- 66 tests, 100% statement/line/function coverage, ~95% branch coverage.
