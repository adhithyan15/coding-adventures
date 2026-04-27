# SQL Query Planner Specification

## Overview

This document specifies the `sql-planner` package: a **query planner** that translates
a parsed SQL AST into a **Logical Plan Tree** — a tree of relational algebra nodes that
captures the *intent* of the query independent of any particular execution strategy or
storage backend.

The planner sits between the parser and the optimizer in the new SQL pipeline:

```
SQL string
    │  sql-lexer + sql-parser
    ▼
AST (ASTNode tree)
    │  sql-planner.plan()
    ▼
LogicalPlan (relational algebra tree)
    │  sql-optimizer  (next stage)
    ▼
OptimizedPlan
    │  sql-codegen
    ▼
IR Bytecode
    │  sql-vm
    ▼
QueryResult
```

**Why a separate planning stage?**

Without a planner, the executor must directly interpret the raw AST. This couples
two very different concerns:

- The **parser** thinks in terms of SQL syntax: `SELECT`, `FROM`, `WHERE`, `GROUP BY`.
- The **executor** thinks in terms of data flow: which tables to scan, how to combine
  rows, how to filter them, how to project output columns.

The Logical Plan is the bridge. It speaks the language of **relational algebra** —
a mathematical theory of sets and operations on them that underpins all relational
databases. By converting to relational algebra first, we gain:

1. A **canonical form** the optimizer can reason about
2. A layer that can represent any SQL statement (SELECT, INSERT, UPDATE, DELETE, DDL)
3. An **independently testable** stage (plan an AST, inspect the tree)

---

## Where It Fits

```
Depends on: sql-parser (for ASTNode types)
Used by:    sql-optimizer (consumes LogicalPlan, returns OptimizedPlan)
```

The planner is **read-only with respect to data** — it never touches a backend. It only
reads the AST and produces a plan tree.

---

## Supported Languages

Implementations exist for all 17 languages in this repository:
`csharp`, `dart`, `elixir`, `fsharp`, `go`, `haskell`, `java`, `kotlin`, `lua`,
`perl`, `python`, `ruby`, `rust`, `starlark`, `swift`, `typescript`, `wasm`.

Each language exposes a single entry-point function:

```
plan(ast: ASTNode) → Result<LogicalPlan, PlanError>
plan_all(statements: ASTNode[]) → Result<LogicalPlan[], PlanError>
```

Language-specific signatures:

**Rust**
```rust
pub fn plan(ast: &ASTNode) -> Result<LogicalPlan, PlanError>
pub fn plan_all(ast: &[ASTNode]) -> Result<Vec<LogicalPlan>, PlanError>
```

**TypeScript**
```typescript
export function plan(ast: ASTNode): LogicalPlan
export function planAll(ast: ASTNode[]): LogicalPlan[]
// throws PlanError on failure
```

**Go**
```go
func Plan(ast *ASTNode) (*LogicalPlan, error)
func PlanAll(stmts []*ASTNode) ([]*LogicalPlan, error)
```

**Python**
```python
def plan(ast: ASTNode) -> LogicalPlan: ...
def plan_all(ast: list[ASTNode]) -> list[LogicalPlan]: ...
# raises PlanError
```

**Ruby**
```ruby
def plan(ast) # → LogicalPlan, raises PlanError
def plan_all(stmts) # → Array<LogicalPlan>
```

**Elixir**
```elixir
@spec plan(ast :: map()) :: {:ok, logical_plan()} | {:error, plan_error()}
@spec plan_all(asts :: [map()]) :: {:ok, [logical_plan()]} | {:error, plan_error()}
```

---

## The Logical Plan Tree

A `LogicalPlan` is a **tree of plan nodes**. Each node represents one relational
algebra operation. Leaf nodes read from tables; internal nodes transform data.

Think of it like a recipe: the leaves are the raw ingredients (table scans), and
each node above them is a cooking step (filter out ingredients you don't need,
combine ingredients, measure out a portion, etc.).

### Node Taxonomy

```
LogicalPlan
├── Scan            — read rows from a named table
├── Filter          — keep rows matching a predicate
├── Project         — select and rename columns
├── Join            — combine rows from two inputs
├── Aggregate       — group rows and compute aggregates
├── Having          — filter after aggregation
├── Sort            — order rows by keys
├── Limit           — take at most N rows, skip M
├── Distinct        — remove duplicate rows
├── Union           — combine two result sets
├── Insert          — insert rows into a table
├── Update          — modify rows in a table
├── Delete          — remove rows from a table
├── CreateTable     — define a new table schema
└── DropTable       — remove a table and its data
```

---

## Node Definitions

### Scan

The leaf node. Reads all rows from a named table. Corresponds to a `FROM table` clause.

```
Scan {
    table:  String          -- the table name as it appears in the query
    alias:  Option<String>  -- AS alias, e.g. FROM employees AS e
}
```

Example: `FROM employees AS e` → `Scan { table: "employees", alias: Some("e") }`

The Scan node does **not** contain column projections or predicates. Those live in
separate Filter and Project nodes above the Scan. Keeping them separate is what
allows the optimizer to push filters *down* to the scan level later.

---

### Filter

Keeps only rows where a boolean expression evaluates to `TRUE`. Rows where the
expression evaluates to `FALSE` or `NULL` are discarded.

```
Filter {
    input:     LogicalPlan  -- the node whose rows are filtered
    predicate: Expr         -- a boolean expression
}
```

Example: `WHERE salary > 50000` wraps its input in:
`Filter { input: Scan{...}, predicate: BinaryOp(Gt, Col("salary"), Lit(50000)) }`

This mirrors the SQL WHERE clause but also appears after GROUP BY (the HAVING clause
produces another Filter node above the Aggregate node).

---

### Project

Selects a specific set of output columns, optionally renaming them. Corresponds to
the `SELECT col1, expr AS alias` list.

```
Project {
    input:   LogicalPlan
    columns: Vec<ProjectionItem>
}

ProjectionItem {
    expr:  Expr            -- any expression (column ref, literal, arithmetic, function)
    alias: Option<String>  -- output column name; if absent, derived from expr
}
```

A `SELECT *` produces `Project { columns: [Wildcard] }` — a special marker that
the code generator expands by asking the backend for column names.

Column alias derivation rules (in priority order):
1. Explicit `AS alias` → use alias
2. Simple column reference `col` → use `col`
3. Qualified reference `table.col` → use `col`
4. Function call `f(...)` → use the function name, e.g. `count`
5. Anything else → use the column index, e.g. `column_1`

---

### Join

Combines rows from two inputs according to a join type and an optional ON condition.

```
Join {
    left:      LogicalPlan
    right:     LogicalPlan
    kind:      JoinKind
    condition: Option<Expr>  -- the ON expression; absent for CROSS JOIN
}

JoinKind = Inner | Left | Right | Full | Cross
```

**Inner join:** only rows where the ON condition is TRUE appear in output.

**Left join:** all rows from `left` appear; unmatched rows get NULL for all `right` columns.

**Right join:** symmetric to Left join.

**Full join:** all rows from both sides; unmatched rows on either side get NULLs.

**Cross join:** cartesian product — every left row paired with every right row.
No ON condition is required or allowed.

Multi-table `FROM t1, t2` (implicit cross join) is planned as:
`Join { left: Scan{t1}, right: Scan{t2}, kind: Cross, condition: None }`

---

### Aggregate

Groups rows and computes aggregate functions over each group.

```
Aggregate {
    input:      LogicalPlan
    group_by:   Vec<Expr>           -- GROUP BY expressions
    aggregates: Vec<AggregateItem>  -- functions to compute per group
}

AggregateItem {
    func:   AggFunc    -- COUNT | SUM | AVG | MIN | MAX
    arg:    AggArg     -- Star (for COUNT(*)) or Expr
    alias:  String     -- output column name
    distinct: bool     -- COUNT(DISTINCT col)
}

AggFunc = Count | Sum | Avg | Min | Max
AggArg  = Star | Expr(Expr)
```

If `group_by` is empty but aggregates are present (e.g. `SELECT COUNT(*) FROM t`),
the entire input is treated as one group.

If neither `group_by` nor aggregates are present, the planner emits no Aggregate node.

---

### Having

A post-aggregation filter. Syntactically identical to a WHERE clause but appears
*after* the Aggregate node in the plan tree.

```
Having {
    input:     LogicalPlan  -- always an Aggregate node
    predicate: Expr
}
```

Example:
```sql
SELECT dept, COUNT(*) AS n FROM employees GROUP BY dept HAVING COUNT(*) > 5
```
Plans as:
```
Having { predicate: BinaryOp(Gt, Col("n"), Lit(5)),
  input: Aggregate { group_by: [Col("dept")], aggregates: [Count(*)→"n"],
    input: Scan { table: "employees" }
  }
}
```

---

### Sort

Orders rows by one or more sort keys.

```
Sort {
    input: LogicalPlan
    keys:  Vec<SortKey>
}

SortKey {
    expr:      Expr
    direction: Asc | Desc
    nulls:     NullsFirst | NullsLast  -- default: NullsLast for ASC, NullsFirst for DESC
}
```

---

### Limit

Restricts the number of output rows. Both `count` and `offset` are optional.

```
Limit {
    input:  LogicalPlan
    count:  Option<u64>  -- LIMIT n; None means unlimited
    offset: Option<u64>  -- OFFSET m; None means 0
}
```

`LIMIT 10 OFFSET 20` skips the first 20 rows and returns at most the next 10.

---

### Distinct

Removes duplicate rows from the output. Two rows are duplicate if every column
value compares equal (NULLs compare equal for the purpose of deduplication).

```
Distinct {
    input: LogicalPlan
}
```

`SELECT DISTINCT` produces a `Distinct` node wrapping the `Project` node.

---

### Union

Combines the results of two queries. `UNION` removes duplicates; `UNION ALL` keeps them.

```
Union {
    left:  LogicalPlan
    right: LogicalPlan
    all:   bool           -- true = UNION ALL; false = UNION (deduplicate)
}
```

Both inputs must have the same number of columns. Column names come from the `left`
side (matching standard SQL behavior).

---

### Insert

Inserts one or more rows into a table.

```
Insert {
    table:   String
    columns: Option<Vec<String>>   -- explicit column list; None = all columns in order
    source:  InsertSource
}

InsertSource = Values(Vec<Vec<Expr>>)   -- INSERT INTO t VALUES (...)
             | Query(LogicalPlan)        -- INSERT INTO t SELECT ...
```

`INSERT INTO employees (name, salary) VALUES ('Alice', 90000), ('Bob', 75000)` plans as:
```
Insert {
    table: "employees",
    columns: Some(["name", "salary"]),
    source: Values([[Lit("Alice"), Lit(90000)], [Lit("Bob"), Lit(75000)]])
}
```

---

### Update

Modifies existing rows in a table.

```
Update {
    table:       String
    assignments: Vec<Assignment>
    predicate:   Option<Expr>      -- WHERE clause; None = update all rows
}

Assignment {
    column: String
    value:  Expr
}
```

---

### Delete

Removes rows from a table.

```
Delete {
    table:     String
    predicate: Option<Expr>  -- WHERE clause; None = delete all rows
}
```

---

### CreateTable

Defines a new table with a schema.

```
CreateTable {
    table:         String
    if_not_exists: bool
    columns:       Vec<ColumnDef>
}

ColumnDef {
    name:         String
    type_name:    String
    not_null:     bool
    primary_key:  bool
    unique:       bool
    default:      Option<Expr>
}
```

---

### DropTable

Removes a table and all of its data.

```
DropTable {
    table:     String
    if_exists: bool
}
```

---

## Expression Types

The `Expr` type is shared across all plan nodes. It mirrors the AST expression nodes
but uses resolved column references (qualified with table alias where unambiguous).

```
Expr =
    | Literal(SqlValue)                          -- 42, 'hello', NULL, TRUE
    | Column(table: Option<String>, col: String) -- employees.salary, salary
    | BinaryOp(BinaryOp, Expr, Expr)
    | UnaryOp(UnaryOp, Expr)
    | FunctionCall(name: String, args: Vec<FuncArg>)
    | IsNull(Expr)
    | IsNotNull(Expr)
    | Between(Expr, low: Expr, high: Expr)
    | In(Expr, Vec<Expr>)
    | NotIn(Expr, Vec<Expr>)
    | Like(Expr, pattern: String)
    | NotLike(Expr, pattern: String)
    | Wildcard                                   -- SELECT *
    | AggregateExpr(AggFunc, AggArg)             -- COUNT(*), SUM(salary)

BinaryOp = Eq | NotEq | Lt | Lte | Gt | Gte | And | Or | Add | Sub | Mul | Div | Mod
UnaryOp  = Not | Neg
FuncArg  = Star | Value(Expr)

SqlValue = Null | Int(i64) | Float(f64) | Text(String) | Bool(bool)
```

---

## Planning Rules

### SELECT statement

A SELECT statement is planned bottom-up, building the tree from leaves to root:

```
1.  Scan / Join tree          ← FROM clause (one Scan per table, Join for JOINs)
2.  Filter(WHERE)             ← WHERE clause wraps step 1
3.  Aggregate                 ← GROUP BY + aggregate functions in SELECT
4.  Having(HAVING)            ← HAVING clause wraps Aggregate
5.  Project(SELECT list)      ← SELECT column list
6.  Distinct                  ← if SELECT DISTINCT
7.  Sort(ORDER BY)            ← ORDER BY clause
8.  Limit(LIMIT/OFFSET)       ← LIMIT / OFFSET
```

Steps 2, 3, 4, 6, 7, 8 are only emitted when the corresponding clause is present.

Example:
```sql
SELECT dept, AVG(salary) AS avg_sal
FROM employees
WHERE active = TRUE
GROUP BY dept
HAVING AVG(salary) > 60000
ORDER BY avg_sal DESC
LIMIT 5
```

Produces:
```
Limit(count=5)
  Sort([avg_sal DESC])
    Having(AVG(salary) > 60000)
      Project([dept, AVG(salary) AS avg_sal])
        Aggregate(group_by=[dept], aggs=[Avg(salary)→"avg_sal"])
          Filter(active = TRUE)
            Scan("employees")
```

### JOIN handling

Each JOIN clause adds a `Join` node above the existing scan tree:

```sql
FROM a JOIN b ON a.id = b.a_id LEFT JOIN c ON b.id = c.b_id
```

Plans as:
```
Join(Left, b.id = c.b_id)
  Join(Inner, a.id = b.a_id)
    Scan("a")
    Scan("b")
  Scan("c")
```

### Multi-table FROM (implicit cross join)

`FROM a, b` is sugar for `FROM a CROSS JOIN b`:

```
Join(Cross)
  Scan("a")
  Scan("b")
```

### DML statements

INSERT, UPDATE, and DELETE are top-level plan nodes — they have no parent.

```sql
UPDATE employees SET salary = salary * 1.1 WHERE dept = 'Engineering'
```
Plans as:
```
Update {
    table: "employees",
    assignments: [salary = salary * 1.1],
    predicate: Some(dept = 'Engineering')
}
```

### DDL statements

CREATE TABLE and DROP TABLE are leaf nodes — they reference no input plan.

---

## Column Resolution

During planning, bare column references are **qualified** where unambiguous.

Given `FROM employees AS e JOIN departments AS d ON e.dept_id = d.id`:
- `salary` → `Column(table: Some("e"), col: "salary")` (only `employees` has it)
- `id` → `AmbiguousColumnError` (both tables have `id`)
- `e.dept_id` → `Column(table: Some("e"), col: "dept_id")` (already qualified)
- `d.name` → `Column(table: Some("d"), col: "name")`

Column resolution requires knowing which columns each table has. The planner
receives a **schema provider** — a simple interface that returns column names
for a given table name.

```
SchemaProvider:
    columns(table: String) → Result<Vec<String>, PlanError>
```

The backend satisfies this interface in full pipeline mode. In unit tests,
the schema provider can be a simple in-memory map.

---

## Error Types

```
PlanError =
    | AmbiguousColumn { column: String, tables: Vec<String> }
    | UnknownTable    { table: String }
    | UnknownColumn   { table: Option<String>, column: String }
    | InvalidAggregate { message: String }
                        -- e.g. aggregate in WHERE clause
    | UnsupportedStatement { kind: String }
    | InternalError   { message: String }
```

---

## Test Harness

The `sql-planner` package ships a shared `conformance` module with helper functions
every language implementation must pass. The helpers take a SQL string and an expected
plan tree and assert structural equality.

Conformance tests cover:
1. Simple SELECT plans to Scan + Project
2. WHERE clause adds Filter between Scan and Project
3. GROUP BY adds Aggregate; HAVING adds Having above it
4. JOIN produces nested Join nodes
5. ORDER BY adds Sort at top
6. LIMIT/OFFSET adds Limit at top
7. SELECT DISTINCT adds Distinct above Project
8. INSERT VALUES produces Insert node
9. UPDATE produces Update node
10. DELETE produces Delete node
11. CREATE TABLE / DROP TABLE produce leaf DDL nodes
12. Ambiguous column reference raises AmbiguousColumn error
13. Unknown table raises UnknownTable error

---

## Relationship to Existing Packages

- **Replaces** the planning logic currently embedded in `sql-execution-engine`.
  The old engine's `_find_first_select` and `execute_select` functions did implicit
  planning and execution in one pass. The new design separates those cleanly.
- **Depends on** `sql-parser` for the `ASTNode` type definitions.
- **Does not depend on** `sql-execution-engine` (the old package can be retired
  once the full new pipeline is in place, or kept as a thin adapter).
