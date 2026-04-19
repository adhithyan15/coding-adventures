# sql-optimizer (Python)

Pure rewrite passes over the `LogicalPlan` tree produced by `sql-planner`.
Each pass takes a plan and returns an equivalent, better-structured plan.

## Where it fits

```
LogicalPlan (from sql-planner)
    │  sql-optimizer.optimize()
    ▼
OptimizedPlan (same LogicalPlan type, restructured)
    │  sql-codegen
    ▼
IR Bytecode
```

## Passes

Run in this order by `optimize()`:

1. **ConstantFolding** — evaluate constant sub-expressions at plan time.
2. **PredicatePushdown** — move filters closer to scans.
3. **ProjectionPruning** — annotate scans with the column subset the query
   actually needs.
4. **DeadCodeElimination** — replace provably-empty subtrees with
   `EmptyResult`; drop always-true filters.
5. **LimitPushdown** — attach a `scan_limit` hint to scans where safe.

Each pass is a pure function `LogicalPlan → LogicalPlan` and each is
idempotent.

## Usage

```python
from sql_optimizer import optimize, Pass, ConstantFolding

optimized = optimize(plan)

# Or run a custom pipeline (useful for tests):
from sql_optimizer import optimize_with_passes
only_folded = optimize_with_passes(plan, [ConstantFolding()])
```

## Errors

The optimizer does not raise user-facing errors. A malformed input plan
raises `sql_planner.InternalError`.

## Development

```
uv venv --clear
uv pip install -e ../sql-backend -e ../sql-planner -e ".[dev]"
.venv/bin/python -m pytest tests/ -v
```

## Specification

See [`code/specs/sql-optimizer.md`](../../../specs/sql-optimizer.md).
