# SQL EXISTS / NOT EXISTS Subquery Expressions

**Spec ID**: sql-exists-subquery  
**Status**: Implemented (v0.8.0 sql-vm / v0.7.0 mini-sqlite)  
**Packages**: `sql-vm`, `sql-codegen`, `sql-planner`, `mini-sqlite`  
**Depends on**: sql-vm v0.7.x (scalar function registry and date/time functions already present)

---

## 1. Motivation

`EXISTS (subquery)` and `NOT EXISTS (subquery)` are among the most common SQL
constructs in real applications.  They test whether a correlated or uncorrelated
subquery produces at least one row:

```sql
-- Find customers who have placed at least one order
SELECT id, name FROM customers
WHERE EXISTS (SELECT 1 FROM orders WHERE orders.customer_id = customers.id);

-- Find products that have never been ordered
SELECT id, name FROM products
WHERE NOT EXISTS (SELECT 1 FROM order_items WHERE order_items.product_id = products.id);

-- Uncorrelated form (subquery always produces the same result)
SELECT * FROM settings
WHERE EXISTS (SELECT 1 FROM feature_flags WHERE name = 'beta_enabled' AND active = 1);
```

Without EXISTS, these queries require awkward rewrites (`LEFT JOIN … IS NULL`,
counting subqueries, etc.) that are harder to read and plan.

---

## 2. Architecture: full pipeline change

Unlike Phase 1 (date/time functions, VM-only), EXISTS affects every layer:

```
SQL text
  → grammar: "EXISTS" "(" query_stmt ")"          ← grammar rule added to primary
  → parser:  primary node with EXISTS + query_stmt children
  → adapter: ExistsSubquery(query=SelectStmt)     ← new expr type (pre-plan)
  → planner: ExistsSubquery(query=LogicalPlan)    ← _resolve plans inner query
  → codegen: RunExistsSubquery(sub_program)       ← new IR instruction
  → vm:      execute sub_program → push TRUE/FALSE ← new VM handler
```

### 2.1 Non-goal: correlated subqueries

Phase 2 implements **uncorrelated EXISTS only**.  A correlated subquery
references a column from the outer query inside the inner SELECT (e.g.
`orders.customer_id = customers.id` above).  Supporting correlated subqueries
requires passing the outer row into the inner VM execution context, which is a
significant architectural change deferred to a later phase.

Attempting a correlated reference will produce an `UnknownColumn` planner error
since the inner query is planned against its own schema scope only.

---

## 3. Grammar changes

The `primary` rule in `sql.grammar` gains one new alternative:

```ebnf
primary = NUMBER | STRING | "NULL" | "TRUE" | "FALSE"
        | case_expr
        | function_call
        | "EXISTS" "(" query_stmt ")"      ← NEW
        | "(" query_stmt ")"
        | column_ref
        | "(" expr ")" ;
```

Ordering matters for PEG parsing: `"EXISTS"` starts with a `KEYWORD` token
whose value is `EXISTS`, so it cannot be confused with `"(" query_stmt ")"` or
`column_ref`.  The new alternative is placed before `"(" query_stmt ")"` to
give the parser a chance to try EXISTS before the generic scalar subquery form.

`NOT EXISTS` uses the existing `not_expr` rule:

```ebnf
not_expr = "NOT" not_expr | comparison ;
```

`NOT EXISTS (...)` parses as `NOT (EXISTS (...))` — the `NOT` keyword wraps
the whole EXISTS primary.  In the adapter this becomes
`UnaryExpr(NOT, ExistsSubquery(...))`, which the existing codegen and VM
already handle correctly via `UnaryOp(NOT)`.

---

## 4. New expression type: `ExistsSubquery`

Added to `sql_planner.expr`:

```python
@dataclass(frozen=True, slots=True)
class ExistsSubquery:
    """EXISTS (subquery) — TRUE iff the inner query returns at least one row.

    Lifecycle
    ---------
    Before ``_resolve``: ``query`` holds the raw ``SelectStmt`` (from the AST
    adapter).  After ``_resolve``: ``query`` holds the fully planned
    ``LogicalPlan`` ready for codegen.

    The ``query`` field is typed as ``object`` to avoid a circular import
    between ``sql_planner.expr`` and ``sql_planner.plan`` (which imports
    ``Expr`` from this module).  Downstream consumers cast it to the
    appropriate type.

    NOT EXISTS
    ----------
    ``NOT EXISTS (...)`` is represented as
    ``UnaryExpr(op=UnaryOp.NOT, operand=ExistsSubquery(...))``.
    No ``negated`` field is needed because the existing ``UnaryOp.NOT``
    instruction handles the inversion at runtime.
    """

    query: object  # SelectStmt before planner; LogicalPlan after _resolve
```

`ExistsSubquery` is added to the `Expr` union and handled in:
- `contains_aggregate()` — returns `False` (the inner query is independently
  aggregated; from the outer expression's perspective there is no aggregate)
- `collect_columns()` — returns nothing (inner columns are independently scoped)

---

## 5. Planner changes

### 5.1 `_resolve()` in `planner.py`

```python
case ExistsSubquery(query=stmt):
    # stmt is a SelectStmt here (pre-resolution).
    # Plan the inner query independently; outer scope is not shared
    # (no correlated subqueries in this phase).
    inner_plan = _plan_select(stmt, schema)
    return ExistsSubquery(query=inner_plan)
```

The inner SELECT is planned against `schema` only — outer table aliases are not
visible inside the subquery.  This is correct for uncorrelated EXISTS.

### 5.2 No optimizer changes needed

The five existing optimizer passes
(`constant_folding`, `dead_code`, `limit_pushdown`, `predicate_pushdown`,
`projection_pruning`) all handle `Filter` nodes by recursing into the predicate
expression.  The new `ExistsSubquery` node in a predicate is transparent: none
of the optimizer passes need to look inside the subquery's plan at this stage.

---

## 6. New IR instruction: `RunExistsSubquery`

Added to `sql_codegen.ir`:

```python
@dataclass(frozen=True, slots=True)
class RunExistsSubquery:
    """Execute inner sub-program; push TRUE iff it returns ≥1 row.

    Unlike ``RunSubquery`` (which materialises rows for cursor-based
    iteration), this instruction is purely a boolean test.  It compiles
    and executes the inner plan in a temporary child state, counts the rows,
    and pushes a single boolean onto the outer expression stack.

    Used for ``EXISTS (subquery)`` expressions in WHERE, HAVING, and
    SELECT projection.  ``NOT EXISTS`` is handled by the caller emitting
    a ``UnaryOp(NOT)`` after this instruction.
    """

    sub_program: Program
```

Added to the `Instruction` union.

---

## 7. Codegen changes

`_compile_expr()` in `compiler.py` gains a new case:

```python
case ExistsSubquery(query=inner_plan):
    # Compile the inner logical plan to a standalone sub-program.
    inner_ctx = _Ctx()
    inner_instrs, _ = _compile_plan(inner_plan, inner_ctx)
    inner_instrs.append(Halt())
    resolved_labels = _resolve_labels(inner_instrs)
    sub = Program(
        instructions=tuple(inner_instrs),
        labels=resolved_labels,
        result_schema=(),
    )
    return [RunExistsSubquery(sub_program=sub)]
```

---

## 8. VM changes

`_do_run_exists_subquery` in `vm.py`:

```python
def _do_run_exists_subquery(ins: RunExistsSubquery, st: _VmState) -> None:
    """Execute sub-program; push TRUE iff ≥1 row was produced."""
    sub_result = execute(ins.sub_program, st.backend)
    st.stack.append(len(sub_result.rows) > 0)
```

Dispatched from the main loop when the instruction is `RunExistsSubquery`.

---

## 9. Version bumps

| Package | Before | After |
|---------|--------|-------|
| `sql-planner` | 0.5.0 | 0.6.0 |
| `sql-codegen` | 0.6.0 | 0.7.0 |
| `sql-vm` | 0.7.0 | 0.8.0 |
| `mini-sqlite` | 0.6.1 | 0.7.0 |

---

## 10. Testing strategy

Tests live in `mini-sqlite/tests/test_tier3_exists.py`.

### Basic EXISTS
- `EXISTS (SELECT 1 FROM t)` returns `TRUE` when t has rows
- `EXISTS (SELECT 1 FROM t)` returns `FALSE` when t is empty
- `EXISTS (SELECT 1 FROM t WHERE col = value)` — filtered subquery

### NOT EXISTS
- `NOT EXISTS (SELECT 1 FROM t)` returns `FALSE` when t has rows
- `NOT EXISTS (SELECT 1 FROM t)` returns `TRUE` when t is empty

### In WHERE clause
- `SELECT … WHERE EXISTS (SELECT 1 FROM lookup WHERE lookup.key = 'x')` — filters correctly
- `SELECT … WHERE NOT EXISTS (SELECT 1 FROM empty_table)` — all rows selected

### NULL propagation
- EXISTS never returns NULL — result is always TRUE or FALSE

### Combining with other predicates
- `WHERE col = 1 AND EXISTS (...)` — AND with EXISTS
- `WHERE EXISTS (...) OR col = 2` — OR with EXISTS

### Edge cases
- Subquery with no FROM (constant subquery): `SELECT 1` — always exists
- Subquery with LIMIT 0: no rows → EXISTS = FALSE
- EXISTS in HAVING clause
- EXISTS in SELECT list: `SELECT EXISTS (SELECT 1 FROM t)` as a value expression

---

## 11. Non-goals

- **Correlated subqueries** — inner query cannot reference outer table columns
- **IN (subquery)** — deferred (raised as `ProgrammingError` by the adapter)
- **Scalar subqueries** — `(SELECT max_val FROM t LIMIT 1)` in expression position, deferred
