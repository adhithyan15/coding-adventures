# SQL Query Optimizer Specification

## Overview

This document specifies the `sql-optimizer` package: a **query optimizer** that takes
a `LogicalPlan` tree (produced by `sql-planner`) and returns an equivalent but more
efficient `LogicalPlan` tree.

The optimizer sits between the planner and the code generator:

```
LogicalPlan (from sql-planner)
    │  sql-optimizer.optimize()
    ▼
OptimizedPlan (same LogicalPlan type, restructured)
    │  sql-codegen  (next stage)
    ▼
IR Bytecode
```

**What "equivalent" means:** two plans are equivalent if they produce the same result
set for any valid backend — same rows, same columns, same ordering (when ORDER BY is
present). The optimizer may not change the semantics of the query, only the structure
of the plan tree.

**Why a separate optimization stage?**

A naive planner emits a correct but inefficient plan. Consider:

```sql
SELECT name FROM employees WHERE dept = 'Engineering'
```

A naive plan scans every employee, filters them, then discards all columns except
`name`. If the employees table has a million rows and only 100 are in Engineering,
the filter runs on all million rows first.

An optimizer can:
- Push the filter *down* so it runs as early as possible (fewer rows to carry)
- Prune columns the query never uses (less memory per row)
- Fold `1 + 1` to `2` at plan time (no arithmetic at execution time)
- Eliminate dead branches like `WHERE FALSE` entirely

Each of these is an **optimization pass**: a pure function `LogicalPlan → LogicalPlan`
that restructures the tree without changing its semantics.

---

## Where It Fits

```
Depends on: sql-planner (for LogicalPlan types)
Used by:    sql-codegen (consumes OptimizedPlan)
```

The optimizer is **purely functional** — it takes a plan and returns a plan. It does
not touch any backend, read any data, or have side effects.

---

## Supported Languages

Implementations exist for all 17 languages in this repository:
`csharp`, `dart`, `elixir`, `fsharp`, `go`, `haskell`, `java`, `kotlin`, `lua`,
`perl`, `python`, `ruby`, `rust`, `starlark`, `swift`, `typescript`, `wasm`.

Entry points:

**Rust**
```rust
pub fn optimize(plan: LogicalPlan) -> LogicalPlan
pub fn optimize_with_passes(plan: LogicalPlan, passes: &[Pass]) -> LogicalPlan
```

**TypeScript**
```typescript
export function optimize(plan: LogicalPlan): LogicalPlan
export function optimizeWithPasses(plan: LogicalPlan, passes: Pass[]): LogicalPlan
```

**Go**
```go
func Optimize(plan *LogicalPlan) *LogicalPlan
func OptimizeWithPasses(plan *LogicalPlan, passes []Pass) *LogicalPlan
```

**Python**
```python
def optimize(plan: LogicalPlan) -> LogicalPlan: ...
def optimize_with_passes(plan: LogicalPlan, passes: list[Pass]) -> LogicalPlan: ...
```

**Ruby**
```ruby
def optimize(plan)         # → LogicalPlan
def optimize_with_passes(plan, passes)  # → LogicalPlan
```

**Elixir**
```elixir
@spec optimize(logical_plan()) :: logical_plan()
@spec optimize_with_passes(logical_plan(), [pass()]) :: logical_plan()
```

---

## Architecture: Composable Passes

An optimization is a **pass** — a single-responsibility tree transformation.
Passes are composable: running pass A then pass B gives a different (and often
better) result than running each alone.

```
type Pass = LogicalPlan → LogicalPlan
```

The default `optimize()` function runs a fixed pipeline of passes in a proven-safe
order. `optimize_with_passes()` lets callers inject a custom pipeline (useful for
testing individual passes in isolation).

The default pass pipeline:

```
1. ConstantFolding          — evaluate compile-time constants
2. PredicatePushdown        — move filters closer to scans
3. ProjectionPruning        — remove unused columns
4. DeadCodeElimination      — remove branches that never produce rows
5. LimitPushdown            — move LIMIT closer to scans where safe
```

Passes are applied **in order, once each**. No fixpoint iteration. If a later pass
creates new opportunities for an earlier pass (rare in this subset), the caller can
re-run the pipeline; this spec does not require convergence loops.

Each pass must be **idempotent**: running it twice on the same input produces the
same result as running it once. This is a correctness requirement — the optimizer
must not oscillate or diverge.

---

## Pass 1: ConstantFolding

**Purpose:** Evaluate expressions whose value can be computed at plan time, before
any data is read.

**Input:** Any plan tree.

**Output:** Same tree with constant sub-expressions replaced by literals.

### Rules

**Arithmetic on literals:**
```
1 + 2       → 3
10 * 4      → 40
'hello' || ' ' || 'world'  → 'hello world'   (string concat)
```

**Comparison of two literals:**
```
1 < 2       → TRUE
'a' = 'b'   → FALSE
NULL = NULL → NULL     (three-valued logic: null comparisons yield NULL, not TRUE)
```

**Boolean simplification:**
```
TRUE AND expr   → expr
FALSE AND expr  → FALSE
TRUE OR expr    → TRUE
FALSE OR expr   → expr
NOT TRUE        → FALSE
NOT FALSE       → TRUE
NOT NULL        → NULL
```

**NULL propagation:**
```
NULL + 5        → NULL
NULL * 0        → NULL     (even though mathematically 0*x=0, SQL says NULL)
COALESCE(NULL, 5) → 5      (function constant-folding)
```

**Division by zero:** `1 / 0` is left as-is (not folded) — the VM raises the error
at runtime so it can carry the original source position for error reporting.

### Traversal

ConstantFolding does a **bottom-up post-order traversal**: fold children before
parents, so that a parent like `1 + (2 + 3)` folds `2 + 3` to `5` first, then
folds `1 + 5` to `6`.

---

## Pass 2: PredicatePushdown

**Purpose:** Move `Filter` nodes as close to the `Scan` leaves as possible, so
fewer rows are carried through the rest of the pipeline.

This is the most impactful optimization in typical OLAP workloads. A filter that
eliminates 99% of rows should run *before* joins that multiply row counts, not after.

### Rules

**Push Filter through Project:**

If a `Filter` node sits above a `Project` node and the predicate only references
columns that are computed directly from the input (not newly-computed expressions),
the filter can move below the project:

```
Before:                         After:
Filter(salary > 50000)          Project(name, salary)
  Project(name, salary)           Filter(salary > 50000)
    Scan(employees)                 Scan(employees)
```

The columns in the predicate must exist in the input of the Project, not just in
its output. If the predicate references an alias created by the Project
(e.g. `HAVING avg_sal > 60000` where `avg_sal` is an alias for `AVG(salary)`),
the filter cannot move below the Project.

**Push Filter through Join:**

A filter whose columns all come from one side of a join can move to that side:

```
Before:
Filter(e.dept = 'Engineering')
  Join(Inner, e.dept_id = d.id)
    Scan(employees AS e)
    Scan(departments AS d)

After:
Join(Inner, e.dept_id = d.id)
  Filter(dept = 'Engineering')   ← pushed to left side
    Scan(employees AS e)
  Scan(departments AS d)
```

**Predicate splitting with AND:**

A conjunction `A AND B` can be split: if `A` can be pushed down but `B` cannot,
push `Filter(A)` down and keep `Filter(B)` at the original level.

```
Before:
Filter(e.dept = 'Eng' AND d.budget > 1000000)
  Join(Inner, e.dept_id = d.id)
    Scan(employees AS e)
    Scan(departments AS d)

After:
Filter(d.budget > 1000000)            ← stays above join
  Join(Inner, e.dept_id = d.id)
    Filter(dept = 'Eng')              ← pushed to employees side
      Scan(employees AS e)
    Scan(departments AS d)
```

**Do not push through Aggregate:**

A filter above an Aggregate (i.e., a HAVING clause) cannot be pushed below it,
because the Aggregate may change the set of rows and column values. The HAVING
clause operates on aggregate results, not raw rows.

**Do not push through Sort or Limit:**

Filters above Sort are safe to push (sorting doesn't change which rows exist).
Filters above Limit are **not** safe to push: `LIMIT 5` then `Filter` is different
from `Filter` then `LIMIT 5` — the filter might see a different 5 rows.

---

## Pass 3: ProjectionPruning

**Purpose:** Remove columns from the plan that are never used by the final output
or any predicate above them. Fewer columns = less memory per row = faster execution.

**Example:**

```sql
SELECT name FROM employees
```

The naive planner emits `Project([name]) over Scan(employees)`, but the Scan
conceptually returns all columns. ProjectionPruning annotates the Scan with a
**required columns** set: `{name}`. The backend only needs to provide those columns.

```
Before:                         After:
Project([name])                 Project([name])
  Scan(employees)                 Scan(employees, required: {name})
```

The `Scan` node gains an optional `required_columns` field (omitted = all columns).

**Propagation rules:**

The pass does a top-down traversal, collecting the set of columns required at each
level:

- `Project` → its required columns = union of all column references in its expressions
- `Filter` → required columns += column references in predicate
- `Join` → required columns for each side = columns from that side referenced
  in the ON condition + columns needed by the parent
- `Aggregate` → required input columns = GROUP BY expressions + aggregate arguments
- `Sort` → required columns += sort key expressions
- `Limit/Distinct` → required columns pass through unchanged

---

## Pass 4: DeadCodeElimination

**Purpose:** Remove plan nodes that provably produce zero rows or that produce
output that is never used.

### Dead Filter (always-false predicate)

If ConstantFolding reduced a filter predicate to the literal `FALSE`, the filter
will never pass any rows. The entire subtree below it is dead.

```
Before:                         After:
Filter(FALSE)                   EmptyResult
  Scan(employees)
```

`EmptyResult` is a special leaf node that the VM executes by returning zero rows
immediately. It has no backend interaction.

### Dead Union branch

If one side of a `Union` is an `EmptyResult`, the Union collapses:

```
Union(all=false)
  EmptyResult
  Scan(orders)
→ Scan(orders)
```

### Limit zero

`LIMIT 0` means "return no rows". Regardless of what is below it:

```
Limit(count=0, offset=0)
  <any subtree>
→ EmptyResult
```

### Always-true filter

If ConstantFolding reduced a predicate to `TRUE`, the filter passes all rows and
can be removed:

```
Filter(TRUE)
  Scan(employees)
→ Scan(employees)
```

---

## Pass 5: LimitPushdown

**Purpose:** Push `Limit` nodes closer to `Scan` nodes where it is safe to do so,
so the backend can short-circuit scanning when enough rows have been read.

**Safe to push through:**
- `Project` — projecting 10 rows is the same as projecting the first 10 rows.
- `Filter` — with a caveat: pushing `LIMIT N` below a `Filter` is only safe if
  the Limit becomes a "hint" (the backend reads at most N rows). Because the Filter
  may reject some of those N rows, the actual output might be fewer than N. The VM
  still applies the real Limit after filtering. This is a **scan limit hint**, not
  a guarantee.

**Not safe to push through:**
- `Sort` — `ORDER BY x LIMIT 5` is completely different from `LIMIT 5` then `ORDER BY`.
  The sort must see all rows before the limit applies.
- `Aggregate` — aggregates consume all rows before producing output.
- `Join` — a join may duplicate or eliminate rows; limiting before the join changes
  which rows are joined.
- `Distinct` — deduplication requires seeing all rows first.

### Scan limit hints

When a Limit is pushed near a Scan, it annotates the Scan with a `scan_limit`:

```
Limit(count=5)
  Filter(dept='Engineering')
    Scan(employees, scan_limit=5)  ← hint: stop after 5 rows pass the filter
```

This is a **hint only** — backends may ignore it. The VM always applies the real
Limit at the correct level.

---

## The EmptyResult Node

A special leaf node introduced by DeadCodeElimination. It has no children and
produces zero rows when executed. It carries no table reference.

```
EmptyResult { columns: Vec<String> }
```

The `columns` field preserves the output schema so downstream nodes can still
type-check correctly.

---

## Pass Interaction Example

This illustrates all five passes working together on a realistic query.

```sql
SELECT e.name
FROM employees AS e
JOIN departments AS d ON e.dept_id = d.id
WHERE e.active = TRUE AND d.name = 'Engineering'
LIMIT 10
```

**After planning (naive):**
```
Limit(10)
  Project([e.name])
    Filter(e.active=TRUE AND d.name='Engineering')
      Join(Inner, e.dept_id=d.id)
        Scan(employees AS e)
        Scan(departments AS d)
```

**Pass 1 – ConstantFolding:** Nothing to fold (no constant sub-expressions).

**Pass 2 – PredicatePushdown:**
- `e.active = TRUE` only references `e` → push to employees side
- `d.name = 'Engineering'` only references `d` → push to departments side
```
Limit(10)
  Project([e.name])
    Join(Inner, e.dept_id=d.id)
      Filter(active=TRUE)
        Scan(employees AS e)
      Filter(d.name='Engineering')
        Scan(departments AS d)
```

**Pass 3 – ProjectionPruning:**
- `Project([e.name])` only needs `name` from employees
- `Join` condition needs `dept_id` from employees and `id` from departments
- Required from employees: `{name, dept_id, active}` (active for the Filter)
- Required from departments: `{id, name}` (name for the Filter)
```
Limit(10)
  Project([e.name])
    Join(Inner, e.dept_id=d.id)
      Filter(active=TRUE)
        Scan(employees AS e, required: {name, dept_id, active})
      Filter(d.name='Engineering')
        Scan(departments AS d, required: {id, name})
```

**Pass 4 – DeadCodeElimination:** Nothing to eliminate (no always-false predicates).

**Pass 5 – LimitPushdown:** Limit cannot push through Join, so stays at top.

Final optimized plan is meaningfully cheaper than the naive one: fewer columns
transferred, filters applied before the join multiplies row counts.

---

## Error Types

The optimizer does not introduce new errors of its own. It is a pure tree rewrite
and cannot fail. If an input plan is valid, the output plan is valid.

However, implementations should guard against malformed input plans (e.g. a Filter
with no input) and surface an `InternalError` rather than panicking.

---

## Test Harness

The `sql-optimizer` package ships a shared `conformance` module.

Conformance tests cover each pass in isolation and in combination:

1. **ConstantFolding:** `1+2` folds to `3`; `TRUE AND FALSE` folds to `FALSE`; `NULL + 5` folds to `NULL`
2. **ConstantFolding:** `NOT TRUE` folds to `FALSE`; `x AND TRUE` simplifies to `x`
3. **PredicatePushdown:** filter above project is pushed below project
4. **PredicatePushdown:** AND predicate splits, each half pushed to the correct join side
5. **PredicatePushdown:** HAVING filter is not pushed below Aggregate
6. **ProjectionPruning:** Scan gains required_columns annotation from Project above it
7. **DeadCodeElimination:** `Filter(FALSE)` becomes `EmptyResult`
8. **DeadCodeElimination:** `Limit(0)` becomes `EmptyResult`
9. **DeadCodeElimination:** `Filter(TRUE)` is removed
10. **LimitPushdown:** `Limit` annotates Scan with scan_limit hint through Project
11. **LimitPushdown:** `Limit` does not push through Sort
12. **Combined:** all five passes applied in sequence yield the correct optimized form

---

## Relationship to Existing Packages

- **Depends on** `sql-planner` for the `LogicalPlan` type definitions.
- The existing `sql-execution-engine` performed no optimization. Optimizations were
  not possible because planning and execution were fused. This package introduces
  optimization as a first-class stage.
