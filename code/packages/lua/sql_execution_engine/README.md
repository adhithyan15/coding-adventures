# sql_execution_engine (Lua)

SELECT-only SQL execution engine with pluggable data sources.

## What it does

Parses and executes SQL SELECT statements against any data source that implements
the two-method DataSource protocol.  The engine is completely self-contained: it
includes a built-in SQL tokenizer and recursive-descent parser so it has no
external dependencies.

## Architecture

The engine uses a **materialized pipeline** model: each stage reads all rows from
the previous stage, transforms them, and passes the result forward.

```
SQL string → Tokenizer → Parser → AST
AST + DataSource →
  Stage 1: FROM + JOIN
  Stage 2: WHERE
  Stage 3: GROUP BY
  Stage 4: HAVING
  Stage 5: SELECT
  Stage 6: DISTINCT
  Stage 7: ORDER BY
  Stage 8: LIMIT / OFFSET
→ { columns, rows }
```

## DataSource protocol

```lua
-- schema(table_name) → list of column-name strings
-- scan(table_name)   → list of row maps { col_name → value }
```

## Usage

```lua
local sql = require("coding_adventures.sql_execution_engine")

local ds = sql.InMemoryDataSource.new({
  employees = {
    { id = 1, name = "Alice", dept = "Engineering", salary = 95000 },
    { id = 2, name = "Bob",   dept = "Marketing",   salary = 72000 },
  },
})

local ok, result = sql.execute(
  "SELECT name, salary FROM employees WHERE salary > 80000 ORDER BY salary DESC",
  ds
)

if ok then
  print(table.concat(result.columns, ", "))
  for _, row in ipairs(result.rows) do
    print(table.concat(row, ", "))
  end
end
```

## Supported SQL

```sql
SELECT [DISTINCT] col1, col2, expr AS alias
FROM table [AS alias]
[INNER | LEFT | RIGHT | FULL | CROSS] JOIN table ON condition
[WHERE expr]
[GROUP BY col1, col2]
[HAVING expr]
[ORDER BY col [ASC | DESC], ...]
[LIMIT n [OFFSET m]]
```

**Aggregate functions**: `COUNT(*)`, `COUNT(col)`, `SUM`, `AVG`, `MIN`, `MAX`

**Expressions**: arithmetic (`+`, `-`, `*`, `/`, `%`), comparisons (`=`, `!=`,
`<>`, `<`, `>`, `<=`, `>=`), `BETWEEN`, `IN`, `LIKE`, `IS NULL`, `IS NOT NULL`,
`AND`, `OR`, `NOT`

**String functions**: `UPPER(s)`, `LOWER(s)`, `LENGTH(s)`

**NULL handling**: three-valued logic — NULL comparisons return NULL (not true/false)

## Dependencies

None beyond `lua >= 5.4`.
