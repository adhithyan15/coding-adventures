# sql-execution-engine

A SELECT-only SQL execution engine for Go. Parses SQL using the `sql-parser` package and executes it against a pluggable `DataSource` interface.

## What it does

This package implements a relational algebra pipeline that takes a SQL SELECT string and a data source, then returns a `QueryResult` with typed rows. It covers:

- **Projection**: `SELECT id, name AS emp_name FROM t`
- **Filtering**: `WHERE salary > 80000 AND active = TRUE`
- **NULL semantics**: three-valued logic (TRUE/FALSE/NULL), `IS NULL`, `IS NOT NULL`
- **Pattern matching**: `LIKE 'A%'`, `LIKE '_ob'`
- **Membership**: `IN (1, 2, 3)`, `NOT IN (...)`
- **Range**: `BETWEEN 70000 AND 90000`
- **Joins**: INNER, LEFT, RIGHT, FULL OUTER, CROSS
- **Aggregates**: `COUNT(*)`, `COUNT(col)`, `SUM`, `AVG`, `MIN`, `MAX`
- **Grouping**: `GROUP BY dept_id HAVING COUNT(*) > 1`
- **Sorting**: `ORDER BY salary DESC, name ASC`
- **Pagination**: `LIMIT 10 OFFSET 20`
- **Deduplication**: `SELECT DISTINCT dept_id FROM employees`
- **Arithmetic**: `salary * 1.1`, `salary + bonus - tax`

## Where it fits in the stack

```
SQL text
    ↓
sql-lexer        — tokenizes SQL keywords, identifiers, operators
    ↓
sql-parser       — builds an AST from the token stream
    ↓
sql-execution-engine   ← YOU ARE HERE
    ↓
DataSource       — your storage backend (in-memory, file, DB)
    ↓
QueryResult      — typed rows ready to display or process
```

## Usage

```go
import sqlengine "github.com/adhithyan15/coding-adventures/code/packages/go/sql-execution-engine"

// Implement the DataSource interface for your storage backend.
type MyDB struct{ /* ... */ }

func (db MyDB) Schema(table string) ([]string, error) {
    // Return ordered column names for the table.
}

func (db MyDB) Scan(table string) ([]map[string]interface{}, error) {
    // Return all rows as maps. Values: nil | int64 | float64 | string | bool
}

// Execute a single SELECT.
result, err := sqlengine.Execute(
    "SELECT name, salary * 1.1 AS raise FROM employees WHERE active = TRUE ORDER BY salary DESC LIMIT 5",
    MyDB{},
)
if err != nil {
    log.Fatal(err)
}
fmt.Println(result) // pretty-prints as an ASCII table

// Or iterate over rows directly.
for i, row := range result.Rows {
    fmt.Printf("Row %d: %v\n", i, row)
}

// Execute multiple statements.
results, err := sqlengine.ExecuteAll(
    "SELECT COUNT(*) FROM employees; SELECT * FROM departments",
    MyDB{},
)
```

## Architecture

The executor is a classic volcano-model pipeline:

```
FROM → JOIN → WHERE → GROUP BY → HAVING → SELECT → DISTINCT → ORDER BY → LIMIT
```

Each stage produces a set of rows consumed by the next stage. This matches how every production SQL database (PostgreSQL, MySQL, SQLite) works at its core.

### Row context

Each row in the pipeline is a `map[string]interface{}` where keys are column names. For JOINs, columns are qualified: `"employees.id"` and `"departments.id"` avoid ambiguity.

### NULL handling

SQL uses three-valued logic. The engine propagates NULL through:
- Arithmetic: `NULL + 5 = NULL`
- Comparison: `NULL = 1 → NULL` (not false — use `IS NULL` instead)
- Boolean: `NULL AND TRUE = NULL`, `NULL AND FALSE = FALSE`

### DataSource contract

| Type | Go type |
|------|---------|
| NULL | `nil` |
| Integer | `int64` |
| Float | `float64` |
| Text | `string` |
| Boolean | `bool` |

## Error types

| Error | When |
|-------|------|
| `*TableNotFoundError` | `Schema()` or `Scan()` returns unknown table |
| `*ColumnNotFoundError` | Column referenced in query doesn't exist |
| `*UnsupportedStatementError` | Non-SELECT statement (INSERT, UPDATE, etc.) |
| `*EvaluationError` | Runtime evaluation failure |

## Limitations

- **SELECT only**: INSERT, UPDATE, DELETE, CREATE TABLE, DROP TABLE return `UnsupportedStatementError`
- **No indexes**: every query is a full table scan; no query optimization
- **No subqueries**: correlated subqueries in WHERE are not supported
- **No window functions**: OVER/PARTITION BY is not implemented
- **Grammar constraint**: CROSS JOIN requires an ON clause (`CROSS JOIN t ON 1 = 1`) because the underlying `sql.grammar` requires ON for all join types

## Dependencies

- `sql-parser` — parses SQL text to AST
- `parser` — generic grammar-driven parser (provides `ASTNode`)
- `lexer` — generic lexer (provides `Token`)
