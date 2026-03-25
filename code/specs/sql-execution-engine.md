# SQL Execution Engine Specification

## Overview

This document specifies the `sql-execution-engine` package: a **SELECT-only** SQL execution
engine that evaluates parsed SQL ASTs against pluggable data sources.

The engine sits one layer above the SQL parser:

```
sql string
    │  sql-parser.parse_sql()
    ▼
AST (ASTNode tree)
    │  sql-execution-engine.execute()
    ▼
QueryResult { columns, rows }
```

The engine is **data-source agnostic** — it knows nothing about CSV, JSON, databases,
or any particular storage format. Data comes in through a `DataSource` protocol/interface/
behaviour that any backend can implement. The first concrete implementation is `sql-csv-source`.

This design mirrors how real databases work: the **query executor** is separate from the
**storage engine**. PostgreSQL, for example, has the same query planner/executor regardless
of whether you use a heap table, a B-tree index, or a foreign data wrapper.

---

## Architecture: The Materialized Pipeline

The engine uses a **materialized pipeline** model: each stage reads all rows from the
previous stage into memory, transforms them, and passes the result to the next stage.

This is simpler than the **volcano/iterator model** (where each operator yields one row
at a time) but educational: you can see the full intermediate state at each pipeline stage.

```
SQL string
    │  parse_sql()
    ▼
AST
    │  execute(ast, data_source)
    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  Stage 1: FROM + JOINs                                                       │
│                                                                             │
│  Scan the base table:  data_source.scan("employees")                        │
│  → [{name: "Alice", dept_id: 1, salary: 90000}, ...]                        │
│                                                                             │
│  For each JOIN clause, scan the joined table and combine via nested loop:   │
│  INNER JOIN departments ON employees.dept_id = departments.id               │
│  → [{name: "Alice", dept_id: 1, salary: 90000,                              │
│      "departments.id": 1, "departments.name": "Engineering"}, ...]          │
└─────────────────────┬───────────────────────────────────────────────────────┘
                      │ [{merged row map}, ...]
                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  Stage 2: WHERE                                                              │
│                                                                             │
│  Keep only rows where the WHERE expression evaluates to true (truthy).       │
│  NULL is treated as false (excluded).                                       │
└─────────────────────┬───────────────────────────────────────────────────────┘
                      │ filtered [{row}]
                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  Stage 3: GROUP BY + Aggregates                                              │
│                                                                             │
│  If GROUP BY is present:                                                    │
│    - Partition rows into groups by the GROUP BY keys                         │
│    - Evaluate aggregate functions (COUNT, SUM, AVG, MIN, MAX) per group     │
│    - Produce one output row per group                                       │
│  If no GROUP BY but aggregates exist in SELECT:                             │
│    - Treat all rows as one group                                            │
│  If no GROUP BY and no aggregates:                                          │
│    - Pass rows through unchanged                                            │
└─────────────────────┬───────────────────────────────────────────────────────┘
                      │ [{row or group row}]
                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  Stage 4: HAVING                                                             │
│                                                                             │
│  Like WHERE but applied after grouping. Filters groups (not individual       │
│  rows) using the HAVING expression. Aggregate functions are allowed here.   │
└─────────────────────┬───────────────────────────────────────────────────────┘
                      │ filtered groups
                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  Stage 5: SELECT (Projection)                                                │
│                                                                             │
│  Evaluate each expression in the SELECT list against the current row.       │
│  Apply AS aliases. Produce the final column names.                          │
│                                                                             │
│  SELECT name, salary * 1.1 AS raised_salary                                 │
│  → [{name: "Alice", raised_salary: 99000.0}, ...]                           │
└─────────────────────┬───────────────────────────────────────────────────────┘
                      │ projected [{col → value}]
                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  Stage 6: DISTINCT                                                           │
│                                                                             │
│  If the SELECT DISTINCT modifier was present, deduplicate projected rows.   │
│  Two rows are equal if all column values are equal.                         │
└─────────────────────┬───────────────────────────────────────────────────────┘
                      │ deduplicated rows
                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  Stage 7: ORDER BY                                                           │
│                                                                             │
│  Sort rows by the ORDER BY expressions.                                     │
│  Default direction is ASC. NULL values sort last in ASC, first in DESC.     │
│  Multi-column sort: primary key first, then secondary, etc.                 │
└─────────────────────┬───────────────────────────────────────────────────────┘
                      │ sorted rows
                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  Stage 8: LIMIT + OFFSET                                                     │
│                                                                             │
│  OFFSET n: skip the first n rows.                                           │
│  LIMIT m: take at most m rows from the remaining set.                       │
└─────────────────────┬───────────────────────────────────────────────────────┘
                      │
                      ▼
             QueryResult { columns: [string], rows: [[value]] }
```

---

## DataSource Protocol

The `DataSource` protocol is the **only interface** between the execution engine and any
storage backend. An engine receives a `DataSource` value at query execution time; it calls
exactly two operations on it:

```
schema(table_name: string) → [column_name: string]
scan(table_name: string)   → [{column_name: string → value}]
```

Where `value` is one of: `string | integer | float | boolean | nil`.

`nil` represents SQL `NULL`. The DataSource is responsible for returning the correct
SQL-typed values — the engine does not coerce strings to numbers; that is the data
source's job.

### Language-Specific Definitions

**Elixir — `@behaviour DataSource`:**
```elixir
@callback schema(table_name :: String.t()) :: [String.t()]
@callback scan(table_name :: String.t()) :: [%{String.t() => term()}]
# Raise TableNotFoundError if table does not exist
```

**Go — `DataSource` interface:**
```go
type DataSource interface {
    Schema(tableName string) ([]string, error)
    Scan(tableName string) ([]map[string]interface{}, error)
}
```

**Python — `DataSource` ABC:**
```python
class DataSource(ABC):
    @abstractmethod
    def schema(self, table_name: str) -> list[str]: ...
    @abstractmethod
    def scan(self, table_name: str) -> list[dict[str, Any]]: ...
```

**Ruby — `DataSource` module (duck-typed interface):**
```ruby
module DataSource
  def schema(table_name) = raise NotImplementedError
  def scan(table_name)   = raise NotImplementedError
end
```

**Rust — `DataSource` trait:**
```rust
pub trait DataSource {
    fn schema(&self, table_name: &str) -> Result<Vec<String>, ExecutionError>;
    fn scan(&self, table_name: &str) -> Result<Vec<HashMap<String, Value>>, ExecutionError>;
}
```

**TypeScript — `DataSource` interface:**
```typescript
interface DataSource {
  schema(tableName: string): string[];
  scan(tableName: string): Record<string, SqlValue>[];
}
// type SqlValue = string | number | boolean | null
```

---

## Public API

### Primary Function

```
execute(sql: string, source: DataSource) → QueryResult
```

Parses `sql` using the `sql-parser` package, then executes the resulting AST against
`source`. Raises/returns error on parse failure or execution error.

### Batch Execution

```
execute_all(sql: string, source: DataSource) → [QueryResult]
```

Parses and executes multiple semicolon-separated statements. Returns one `QueryResult`
per statement.

### Language-Specific Signatures

| Language | Signature |
|----------|-----------|
| Elixir | `execute(sql, source)` → `{:ok, %QueryResult{}}` \| `{:error, reason}` |
| Go | `Execute(sql string, source DataSource) (*QueryResult, error)` |
| Python | `execute(sql: str, source: DataSource) -> QueryResult` |
| Ruby | `execute(sql, source)` → `QueryResult` |
| Rust | `pub fn execute(sql: &str, source: &dyn DataSource) -> Result<QueryResult, ExecutionError>` |
| TypeScript | `execute(sql: string, source: DataSource): QueryResult` |

---

## QueryResult

```
QueryResult {
  columns: [string]      // display names in SELECT order
  rows:    [[value]]     // each row is a list of values in column order
}
```

**Column naming rules:**
1. If the select item has an `AS alias`, the column name is `alias`.
2. If `SELECT *` and a single table is scanned, the column names are the table's schema column names.
3. If `SELECT *` across a join, column names are `table.column` (qualified).
4. If the select item is a bare `column_ref` (single-part name) with no alias, the column name is the column name.
5. If the select item is a bare `table.column` reference, the column name is `column` (unqualified).
6. If the select item is an expression (arithmetic, function call, etc.) with no alias, the column name is the expression text as-is (implementation may simplify; tests should use aliases).

---

## Expression Evaluation

Expressions appear in WHERE, HAVING, SELECT list, JOIN ON conditions, ORDER BY, and
HAVING clauses. All are evaluated by the same recursive `eval_expr` function.

### Evaluation Context

Each expression is evaluated against a **row context**: a flat map from column name to
value. In a join, columns from all tables are merged into one map. Ambiguous names
(same column in two joined tables) are qualified: `table.column`.

### Literal Values

| AST token | Elixir/Python value | Notes |
|-----------|---------------------|-------|
| `NUMBER "42"` | `42` (integer) | If no `.` in value |
| `NUMBER "3.14"` | `3.14` (float) | If `.` present |
| `STRING "hello"` | `"hello"` | Lexer already strips quotes |
| `NULL` keyword | `nil` / `None` / `null` | SQL null |
| `TRUE` keyword | `true` | Boolean |
| `FALSE` keyword | `false` | Boolean |

### Column References

```
column_ref = NAME [ "." NAME ]
```

Single-part `name`:
- Look up `row["name"]`; if not found, try `row["table.name"]` for all tables in context
- If found in exactly one table: return that value
- If found in multiple tables: `AmbiguousColumnError`
- If found in no table: `ColumnNotFoundError`

Two-part `table.column`:
- Look up `row["table.column"]` directly
- If not found: `ColumnNotFoundError`

### Arithmetic Operators

| Operator | AST rule | Behaviour |
|----------|----------|-----------|
| `+` | `additive` | Add numbers; string concat not supported |
| `-` | `additive` | Subtract numbers |
| `*` | `multiplicative` | Multiply |
| `/` | `multiplicative` | Divide; `DivisionByZeroError` if divisor is 0 |
| `%` | `multiplicative` | Modulo; `DivisionByZeroError` if divisor is 0 |
| unary `-` | `unary` | Negate a number |

**NULL propagation:** Any arithmetic on `NULL` returns `NULL`. Example: `NULL + 5` → `NULL`.

**Type mismatch:** Adding a string to a number raises `TypeMismatchError`.

**Integer vs float:** If both operands are integers, integer arithmetic is used. If either
is a float, the result is a float. Division always produces a float in languages where
integer division truncates.

### Comparison Operators

| Operator | Notes |
|----------|-------|
| `=` | SQL equality; `NULL = NULL` → `NULL` (not `true`) |
| `!=` / `<>` | Not equal; `NULL != NULL` → `NULL` |
| `<`, `>`, `<=`, `>=` | Numeric or string lexicographic ordering; NULL propagates |

**NULL propagation:** Any comparison involving `NULL` returns `NULL`, except `IS NULL`
and `IS NOT NULL`.

### IS NULL / IS NOT NULL

```
expr IS NULL     → true  if expr evaluates to nil/null
expr IS NOT NULL → false if expr evaluates to nil/null
```

These are the only operators that return `true` or `false` for NULL inputs.

### BETWEEN

```
expr BETWEEN low AND high
```

Equivalent to `expr >= low AND expr <= high`, but evaluated atomically. NULL in any
operand returns NULL. The bounds are inclusive.

```
expr NOT BETWEEN low AND high
```

Equivalent to `NOT (expr BETWEEN low AND high)`.

### IN

```
expr IN (v1, v2, ..., vn)
```

Returns `true` if `expr` equals any value in the list. Returns `false` if no match and
`expr` is not NULL. Returns NULL if `expr` is NULL.

Note: if any `vi` is NULL and `expr` does not match any non-NULL value, the result is
NULL (not false), because `expr = NULL` is NULL.

```
expr NOT IN (v1, v2, ..., vn)
```

Equivalent to `NOT (expr IN (v1, v2, ..., vn))`.

### LIKE

```
expr LIKE pattern
expr NOT LIKE pattern
```

Pattern matching on strings. Two wildcard characters:
- `%` — matches zero or more characters
- `_` — matches exactly one character

All other characters match literally (case-sensitive).

Examples:
```
'hello' LIKE 'h%'      → true
'hello' LIKE 'h_llo'   → true
'hello' LIKE 'H%'      → false   (case-sensitive)
'hello' LIKE '%llo'    → true
'hello' LIKE 'h___o'   → true
'hi'    LIKE 'h___o'   → false
NULL    LIKE '%'       → NULL
```

Implementation: convert LIKE pattern to a regex. `%` → `.*`, `_` → `.`, literal chars
are regex-escaped.

### Three-Valued Boolean Logic (AND, OR, NOT)

SQL uses three-valued logic: `true`, `false`, and `NULL` (unknown).

**AND truth table:**

| A     | B     | A AND B |
|-------|-------|---------|
| true  | true  | true    |
| true  | false | false   |
| true  | NULL  | NULL    |
| false | true  | false   |
| false | false | false   |
| false | NULL  | false   |
| NULL  | true  | NULL    |
| NULL  | false | false   |
| NULL  | NULL  | NULL    |

**OR truth table:**

| A     | B     | A OR B |
|-------|-------|--------|
| true  | true  | true   |
| true  | false | true   |
| true  | NULL  | true   |
| false | true  | true   |
| false | false | false  |
| false | NULL  | NULL   |
| NULL  | true  | true   |
| NULL  | false | NULL   |
| NULL  | NULL  | NULL   |

**NOT truth table:**

| A     | NOT A |
|-------|-------|
| true  | false |
| false | true  |
| NULL  | NULL  |

**WHERE/HAVING filter rule:** A row passes the filter if and only if the expression
evaluates to `true`. Rows where the expression evaluates to `false` OR `NULL` are excluded.

---

## JOIN Semantics

All joins are implemented as **nested loop joins**: for each row in the left set, scan
all rows in the right set and test the join condition. This is O(n×m) but simple and
correct.

### Column Qualification in Joins

When two tables are joined, column names must be qualified to avoid ambiguity. The engine
maintains a map of `{alias → [column_name]}` for all tables in scope.

After joining `employees AS e` and `departments AS d`, the row context map looks like:
```
{
  "e.id": 1, "e.name": "Alice", "e.dept_id": 2,
  "d.id": 2, "d.name": "Engineering",
}
```

Unqualified column references are resolved by searching all tables. Ambiguous references
(same column name in two tables) raise `AmbiguousColumnError`.

### INNER JOIN

```sql
employees INNER JOIN departments ON employees.dept_id = departments.id
```

Result: only rows where the ON condition is `true`. Rows with NULL ON condition are
excluded (same as WHERE filtering).

```
left_rows  = [{e.id:1, e.name:"Alice", e.dept_id:2}, {e.id:2, e.name:"Bob", e.dept_id:NULL}]
right_rows = [{d.id:1, d.name:"HR"}, {d.id:2, d.name:"Engineering"}]

Combined:
  Alice + HR          → ON: 2 = 1 → false, excluded
  Alice + Engineering → ON: 2 = 2 → true,  INCLUDED
  Bob   + HR          → ON: NULL = 1 → NULL, excluded
  Bob   + Engineering → ON: NULL = 2 → NULL, excluded

Result: [{Alice, dept_id:2, Engineering}]
```

### LEFT JOIN

Every row from the left side appears in the result, even if there is no matching right
row. Unmatched right columns are filled with `NULL`.

```
left_rows  = [Alice(dept_id:2), Bob(dept_id:NULL)]
right_rows = [HR(id:1), Engineering(id:2)]

Alice → matches Engineering → output {Alice, Engineering}
Bob   → no match (NULL ON NULL) → output {Bob, d.id:NULL, d.name:NULL}
```

### RIGHT JOIN

Mirror of LEFT JOIN: every row from the right side appears. Unmatched left columns are
`NULL`.

Implemented as: swap left and right, perform LEFT JOIN, swap columns back.

### FULL OUTER JOIN

Union of LEFT JOIN and RIGHT JOIN results, with duplicates eliminated.

Implemented as: LEFT JOIN ∪ RIGHT JOIN minus rows that appear in both (the matched rows
that INNER JOIN would have produced are already in LEFT JOIN).

### CROSS JOIN

Cartesian product. No ON condition. Every left row combined with every right row.

```
left = [A, B]   right = [1, 2]
Result: [A1, A2, B1, B2]
```

---

## GROUP BY and Aggregation

### Grouping

When `GROUP BY col1, col2` is present:
1. Evaluate the GROUP BY expressions for each row to produce a **group key** (a tuple of values).
2. Partition the rows into groups by this key.
3. For each group, evaluate aggregate functions.

### Aggregate Functions

Aggregate functions operate on a set of rows (a group) rather than a single row.

| Function | Behaviour | NULL handling |
|----------|-----------|---------------|
| `COUNT(*)` | Count all rows in group | NULLs included |
| `COUNT(expr)` | Count rows where expr is not NULL | NULLs excluded |
| `SUM(expr)` | Sum of all non-NULL expr values | NULLs excluded; empty group → NULL |
| `AVG(expr)` | Average of all non-NULL values | NULLs excluded; empty group → NULL |
| `MIN(expr)` | Minimum non-NULL value | NULLs excluded; empty group → NULL |
| `MAX(expr)` | Maximum non-NULL value | NULLs excluded; empty group → NULL |

**Empty group edge case:** A group with all-NULL values for `SUM`/`AVG`/`MIN`/`MAX` returns
`NULL`, not `0`. `COUNT(expr)` returns `0`.

### Two-Pass Evaluation

Aggregate expressions in SELECT cannot be evaluated row-by-row — they need the full group.
The execution proceeds in two passes:

**Pass 1:** Identify which select items contain aggregate functions. Evaluate all
non-aggregate expressions at row level. Collect all rows per group.

**Pass 2:** For each group, compute aggregate values and combine with non-aggregate values
(which must be the same for all rows in the group, as they are GROUP BY keys or constants).

### Implicit Grouping

If the SELECT list contains aggregate functions but there is no GROUP BY clause, treat
all rows as a single group. Example:

```sql
SELECT COUNT(*), AVG(salary) FROM employees
```

Returns one row with the total count and average salary.

### Aggregate Detection

An expression contains an aggregate function if any `function_call` node in the expression
tree has a name matching `COUNT`, `SUM`, `AVG`, `MIN`, or `MAX` (case-insensitive, as the
lexer normalizes to uppercase).

---

## ORDER BY

```sql
ORDER BY expr [ASC | DESC] {, expr [ASC | DESC]}
```

- Default direction is `ASC`.
- Multi-column sort: primary sort on first expression, ties broken by second, etc.
- `NULL` values sort **last** in `ASC` order, **first** in `DESC` order. (This matches
  PostgreSQL behavior and is the most common convention.)
- String comparison: lexicographic, by Unicode code point (locale-independent for simplicity).
- Number comparison: numeric value (integers and floats compared by value).
- Mixed-type comparison (string vs number): `TypeMismatchError`.

---

## DISTINCT

```sql
SELECT DISTINCT col1, col2 FROM ...
```

After projection, remove duplicate rows. Two rows are duplicates if all their values
are equal (using SQL equality: `NULL` does NOT equal `NULL` for this purpose — two rows
with all-NULL values are NOT considered equal and both appear in the output).

Wait — this is a pragmatic implementation choice. Real SQL DISTINCT treats two NULLs as
the same for deduplication. We follow that convention: `NULL IS NOT DISTINCT FROM NULL`
in the DISTINCT context, so two rows both having NULL in the same column are duplicates.

---

## LIMIT and OFFSET

```sql
LIMIT 10 OFFSET 5
```

- `OFFSET n`: skip the first `n` rows (0-indexed). Default: 0.
- `LIMIT m`: return at most `m` rows from the remaining set.
- `LIMIT 0`: returns no rows.
- `LIMIT` without `OFFSET`: skip 0 rows.
- Applying `OFFSET` larger than total row count: returns empty result.

---

## Error Semantics

| Error type | When raised |
|-----------|-------------|
| `ParseError` | The SQL string fails to parse |
| `TableNotFoundError` | `data_source.scan(name)` is called for an unknown table |
| `ColumnNotFoundError` | A column reference resolves to no column in the row context |
| `AmbiguousColumnError` | An unqualified column name matches columns in 2+ joined tables |
| `TypeMismatchError` | Arithmetic or comparison on incompatible types (e.g., string + number) |
| `DivisionByZeroError` | Division or modulo by zero |
| `ExecutionError` | Catch-all for unexpected conditions |

---

## In-Memory DataSource (for Testing)

The `sql-execution-engine` package itself must NOT depend on `csv-parser`. Tests use an
**in-memory DataSource** that holds hardcoded data:

```
# Elixir example
defmodule InMemorySource do
  @behaviour DataSource

  def schema("employees"),   do: ["id", "name", "dept_id", "salary"]
  def schema("departments"), do: ["id", "name"]
  def schema(_),             do: raise TableNotFoundError

  def scan("employees"), do: [
    %{"id" => 1, "name" => "Alice", "dept_id" => 1, "salary" => 90000},
    %{"id" => 2, "name" => "Bob",   "dept_id" => 2, "salary" => 75000},
    %{"id" => 3, "name" => "Carol", "dept_id" => 1, "salary" => 95000},
  ]
  ...
end
```

This keeps the `sql-execution-engine` package lean (no CSV dependency) and lets its
tests run without file I/O.

---

## Supported SQL Subset

The engine executes the SQL subset defined in `sql.md`. Summary:

| Feature | Supported |
|---------|-----------|
| `SELECT *` | ✓ |
| `SELECT expr AS alias` | ✓ |
| `SELECT DISTINCT` | ✓ |
| `FROM table` | ✓ |
| `FROM table AS alias` | ✓ |
| INNER / LEFT / RIGHT / FULL / CROSS JOIN | ✓ |
| `WHERE` with full expression support | ✓ |
| `GROUP BY` | ✓ |
| `HAVING` | ✓ |
| `ORDER BY` (multi-column, ASC/DESC) | ✓ |
| `LIMIT` / `OFFSET` | ✓ |
| `COUNT`, `SUM`, `AVG`, `MIN`, `MAX` | ✓ |
| Subqueries | ✗ (not in SQL grammar) |
| `INSERT`, `UPDATE`, `DELETE` | ✗ (read-only engine) |
| `CREATE TABLE`, `DROP TABLE` | ✗ (read-only engine) |
| Window functions | ✗ |
| CTEs (`WITH`) | ✗ |

---

## Relationship to Other Packages

| Package | Relationship |
|---------|-------------|
| `sql-parser` | Provides `parse_sql()` which this engine calls to get the AST |
| `sql-lexer` | Transitively required by `sql-parser` |
| `csv-parser` | **NOT a dependency.** CSV integration is in `sql-csv-source`. |
| `sql-csv-source` | Implements `DataSource` using `csv-parser`. Separate package. |

---

## Relationship to Other Specs

| Spec | Relationship |
|------|-------------|
| `sql.md` | Defines the SQL grammar and AST node types consumed by this engine |
| `csv-parser.md` | The CSV parser spec — used by `sql-csv-source`, not this engine |
| `03-parser.md` | Base parser infrastructure that `sql-parser` builds on |
