# sql-optimizer (TypeScript)

SQL query optimizer: applies rewrite passes to a `LogicalPlan` tree (from
`@coding-adventures/sql-planner`) and returns a semantically equivalent,
potentially more efficient plan.

## Where It Fits

```
sql-parser → sql-planner → sql-optimizer → sql-codegen → sql-vm
                                 ↑
                            this package
```

## Usage

```typescript
import { optimize } from "@coding-adventures/sql-optimizer";

// `logical` is a LogicalPlan from sql-planner
const optimized = optimize(logical);
```

## Architecture

The optimizer is a **pure function**: it takes a `LogicalPlan` tree and
returns an equivalent tree. It never executes SQL and never touches the
database. This makes it safe to call multiple times, easy to test in
isolation, and straightforward to port to other languages.

Each optimization pass performs a single tree transformation. Passes are
composed sequentially: the output of one pass is the input to the next.

### Optimization passes (v0.1.0)

| Pass | What it does |
|------|--------------|
| Predicate pushdown | Moves `FilterNode` predicates closer to the scan source to reduce rows processed |
| Constant folding | Evaluates constant sub-expressions at plan time (`1+1 → 2`, `NOT TRUE → FALSE`) |
| Dead projection elimination | Removes `ProjectNode` items not referenced by ancestor nodes |

The optimizer returns the input plan unchanged when no rewrites apply —
it is safe to call even on plans that need no optimization.

### Extension

To add a new optimization pass, add a function `transformXxx(plan: LogicalPlan): LogicalPlan`
and include it in the `optimize` composition chain in `optimizer.ts`.
