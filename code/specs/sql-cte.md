# SQL Common Table Expressions (Non-Recursive WITH)

**Spec ID**: sql-cte
**Status**: Implemented (v1.2.0 sql-vm / v1.1.0 mini-sqlite)
**Packages**: `sql-vm`, `sql-codegen`, `sql-backend`, `sql-lexer`, `sql-parser`, `mini-sqlite`
**Depends on**: sql-foreign-keys (Phase 4b, already merged)

---

## 1. Motivation

A Common Table Expression (CTE) is a named temporary result set defined
within a single SQL statement via the `WITH` clause:

```sql
WITH recent_orders AS (
    SELECT id, customer_id, total
    FROM orders
    WHERE created_at > '2025-01-01'
)
SELECT c.name, SUM(o.total) AS revenue
FROM customers AS c
JOIN recent_orders AS o ON c.id = o.customer_id
GROUP BY c.name;
```

CTEs improve readability by giving a meaningful name to a subquery that
would otherwise appear anonymously in a FROM clause.  They also act as a
query-composition primitive — the CTE can be referenced multiple times in
the outer query without repeating its body.

---

## 2. SQL semantics

### 2.1 Syntax

```
WITH cte_name AS ( query_stmt )
[ , cte_name2 AS ( query_stmt2 ) ]
...
select_stmt
```

This spec implements **non-recursive** CTEs only.  The `RECURSIVE` keyword
is not supported (it is deferred to Phase 5b).

### 2.2 Scoping

A CTE is visible only within the `SELECT` statement that immediately follows
the WITH clause.  In the outer query, the CTE name may appear:

- In the `FROM` clause (primary table reference).
- In a `JOIN` clause (right-hand side).

### 2.3 Multiple CTEs

Multiple CTEs may be defined in a single WITH clause, separated by commas:

```sql
WITH
    a AS (SELECT ...),
    b AS (SELECT ... FROM a)   -- b can reference a
SELECT * FROM b;
```

Later CTEs may reference earlier ones because the adapter resolves each CTE
body in declaration order and makes it available to subsequent bodies and
the main query.

### 2.4 Duplicate references

If the outer query references the same CTE name more than once (e.g., in a
self-join), the CTE's inner query is re-executed for each reference.  This
is semantically correct, though not optimal.  Memoization is deferred to
Phase 5b.

### 2.5 NULL and aggregate semantics

CTEs follow ordinary SELECT rules. Any valid SELECT statement is a valid
CTE body (including DISTINCT, GROUP BY, HAVING, ORDER BY, LIMIT, JOIN).

---

## 3. Grammar changes

### 3.1 New keyword: `WITH`

`WITH` is added to `sql.tokens` and the compiled `keywords` list in
`sql_lexer/_grammar.py`.

### 3.2 `query_stmt` extension

```
query_stmt  = [ with_clause ] select_stmt { set_op_clause } ;
with_clause = "WITH" cte_def { "," cte_def } ;
cte_def     = NAME "AS" "(" query_stmt ")" ;
```

`with_clause` is `Optional` at the start of `query_stmt`, so plain queries
(`SELECT …`) parse unchanged.

The grammar remains unambiguous because PEG parsers are greedy: when the
parser sees `WITH`, it attempts the `with_clause` rule first.  If the
following token is not a valid `cte_def`, it backtracks.  Because `WITH` is
now a reserved keyword (not matched as a table name), the ambiguity class is
eliminated.

---

## 4. Pipeline changes

### 4.1 sql-lexer: `WITH` keyword

`WITH` is added to the `keywords` list in the compiled `_grammar.py`.

### 4.2 sql-parser: compiled grammar

Three changes in `_grammar.py`:

1. **`query_stmt`** — wraps `select_stmt` with a leading
   `Optional(RuleReference('with_clause'))`.
2. **`with_clause`** — new rule: `"WITH" cte_def { "," cte_def }`.
3. **`cte_def`** — new rule: `NAME "AS" "(" query_stmt ")"`.

### 4.3 mini-sqlite adapter: CTE resolution

CTEs are **fully resolved in the adapter layer**.  No new planner AST or
plan nodes are needed: each CTE reference is rewritten to a `DerivedTableRef`
before the planner sees it.

#### `_query_stmt` changes

Before dispatching to `_select`, the adapter extracts the optional
`with_clause` child and builds a `ctes` dict:

```python
ctes: dict[str, SelectStmt] = {}
with_node = _maybe_child(node, "with_clause")
if with_node is not None:
    for cte_node in _child_nodes(with_node, "cte_def"):
        name = _first_token(cte_node, kind="NAME").value
        inner_stmt = _query_stmt(_child_node(cte_node, "query_stmt"))
        ctes[name] = inner_stmt   # earlier CTEs become available to later ones
```

The `ctes` dict is then passed into `_select(node, ctes=ctes)`.

#### `_select` changes

Accepts an optional `ctes` parameter and forwards it to `_table_ref` and
`_join_clause`.

#### `_table_ref` changes

When the grammar rule has no `query_stmt` child (plain table reference), the
function checks whether the table name is in the `ctes` dict before returning
a `TableRef`:

```python
# Plain table — check if it resolves to a CTE.
if ctes and table in ctes:
    return DerivedTableRef(select=ctes[table], alias=alias or table)
return TableRef(table=table, alias=alias)
```

If the name is in `ctes`, a `DerivedTableRef` is returned instead of a
`TableRef`.  The alias defaults to the CTE name when no explicit `AS alias`
is given, which is standard SQL behaviour.

#### No changes below the adapter

Because the adapter converts CTE references to `DerivedTableRef`, the
planner, codegen, and VM see exactly the same structure as a derived table
(subquery in FROM).  The planner routes `DerivedTableRef → P.DerivedTable`,
the codegen emits `RunSubquery`, and the VM executes the inner program and
materialises the rows as a `_SubqueryCursor`.  No new IR instructions or
VM handlers are required.

### 4.4 sql-vm / sql-codegen / sql-backend

No changes.  CTE support comes for free through the existing derived-table
machinery.

---

## 5. Error handling

| Error condition | Behaviour |
|---|---|
| CTE name used but not defined | `ProgrammingError("expected child rule …")` or `UnknownColumn`/`UnknownTable` from planner |
| CTE body is a set operation | `ProgrammingError("CTE body must be a plain SELECT, not a set operation")` |
| CTE alias omitted from FROM | The CTE name itself becomes the implicit alias |
| Recursive CTE (`WITH RECURSIVE`) | `RECURSIVE` is not a keyword; it appears as a NAME token; the grammar parses `RECURSIVE` as the CTE name, which fails at execution |

---

## 6. Limitations and future work

- **`RECURSIVE`** — required for hierarchical / graph queries (Phase 5b).
- **Memoization** — CTE bodies referenced multiple times re-execute each
  time.  A materialise-once cache can be added in Phase 5b.
- **Set-operation CTE bodies** — `WITH cte AS (SELECT … UNION SELECT …)`
  is not supported (Phase 5b).
- **CTEs in DML** — `WITH … INSERT/UPDATE/DELETE` is out of scope.
- **Column-name lists** — `WITH cte(a, b) AS (…)` is not supported.

---

## 7. Test strategy

### Grammar / parser unit tests (`sql-parser`)

| Test | What it verifies |
|---|---|
| `test_grammar_parses_cte_basic` | `WITH c AS (SELECT …) SELECT …` produces a valid AST |
| `test_grammar_parses_cte_no_column` | CTE without explicit column list |
| `test_grammar_parses_multiple_ctes` | Two CTEs in one WITH clause |

### Integration tests (`mini-sqlite/test_tier3_cte.py`)

| Test | What it verifies |
|---|---|
| `test_cte_basic` | CTE result rows accessible in outer query |
| `test_cte_with_filter` | CTE WITH WHERE clause |
| `test_cte_with_aggregation` | CTE WITH GROUP BY / COUNT |
| `test_cte_multiple` | Multiple CTEs, later references earlier |
| `test_cte_joined_with_real_table` | CTE joined with a base table |
| `test_cte_column_alias_preserved` | SELECT col AS alias in CTE body |
| `test_cte_referenced_twice` | Same CTE in FROM and JOIN — both materialise |
| `test_cte_outer_alias` | `FROM cte AS c` — outer AS alias |
| `test_cte_unknown_table_still_fails` | Unrelated OperationalError unchanged |
| `test_no_cte_query_unaffected` | Plain SELECT still works |
