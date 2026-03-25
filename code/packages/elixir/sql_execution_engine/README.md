# sql_execution_engine

A SELECT-only SQL execution engine for Elixir.

This package sits at the top of the SQL processing stack. It takes a SQL
string, parses it via `sql_parser`, and executes it against a pluggable
`DataSource` behaviour — delivering structured query results.

## Stack position

```
sql_execution_engine   ← you are here
      │
      ▼
  sql_parser
      │
      ├── sql_lexer
      ├── grammar_tools
      ├── parser
      └── lexer
```

## Usage

```elixir
defmodule MySource do
  @behaviour CodingAdventures.SqlExecutionEngine.DataSource

  @impl true
  def schema("users"), do: ["id", "name", "email"]

  @impl true
  def scan("users") do
    [
      %{"id" => 1, "name" => "Alice", "email" => "alice@example.com"},
      %{"id" => 2, "name" => "Bob",   "email" => "bob@example.com"},
    ]
  end
end

alias CodingAdventures.SqlExecutionEngine

{:ok, result} = SqlExecutionEngine.execute("SELECT * FROM users", MySource)
result.columns  # => ["id", "name", "email"]
result.rows     # => [[1, "Alice", "alice@example.com"], [2, "Bob", "bob@example.com"]]
```

## Supported SQL

| Feature            | Example                                              |
|--------------------|------------------------------------------------------|
| Full scan          | `SELECT * FROM employees`                            |
| Projection         | `SELECT id, name FROM employees`                     |
| Column alias       | `SELECT name AS employee_name FROM employees`        |
| Table alias        | `FROM employees AS e`                                |
| WHERE              | `WHERE salary > 80000`                               |
| IS NULL / NOT NULL | `WHERE dept_id IS NULL`                              |
| BETWEEN            | `WHERE salary BETWEEN 70000 AND 90000`               |
| IN                 | `WHERE id IN (1, 3)`                                 |
| LIKE               | `WHERE name LIKE 'A%'`                               |
| AND / OR / NOT     | `WHERE active = true AND salary > 80000`             |
| INNER JOIN         | `INNER JOIN departments AS d ON e.dept_id = d.id`   |
| LEFT JOIN          | `LEFT JOIN departments AS d ON e.dept_id = d.id`    |
| RIGHT JOIN         | `RIGHT JOIN departments AS d ON e.dept_id = d.id`   |
| FULL JOIN          | `FULL JOIN departments AS d ON e.dept_id = d.id`    |
| CROSS JOIN         | `CROSS JOIN departments AS d ON 1 = 1`              |
| GROUP BY           | `GROUP BY dept_id`                                   |
| HAVING             | `HAVING SUM(salary) > 100000`                        |
| Aggregates         | `COUNT(*)`, `SUM(col)`, `AVG(col)`, `MIN`, `MAX`    |
| ORDER BY           | `ORDER BY salary DESC`                               |
| LIMIT / OFFSET     | `LIMIT 10 OFFSET 5`                                  |
| DISTINCT           | `SELECT DISTINCT dept_id FROM employees`             |
| Arithmetic         | `SELECT salary * 1.1 AS raised FROM employees`       |

## DataSource behaviour

To plug in a custom data source, implement two callbacks:

```elixir
@callback schema(table_name :: String.t()) :: [String.t()]
@callback scan(table_name :: String.t()) :: [%{String.t() => term()}]
```

`schema/1` returns the ordered list of column names.
`scan/1` returns all rows as maps with string keys.

Raise `TableNotFoundError` for unknown table names.

## Execution pipeline

Queries execute in SQL's logical order:

1. **FROM** — scan the base table
2. **JOIN** — extend rows with joined tables (nested-loop)
3. **WHERE** — filter rows (three-valued NULL logic)
4. **GROUP BY** — group and compute aggregates
5. **HAVING** — filter groups
6. **SELECT** — project columns and evaluate expressions
7. **DISTINCT** — remove duplicate rows
8. **ORDER BY** — sort output
9. **LIMIT/OFFSET** — slice output

## NULL handling

SQL uses three-valued logic for NULL. Any comparison involving NULL yields
NULL (unknown), not TRUE or FALSE. The engine correctly implements:

- `NULL = NULL` → NULL (not TRUE!)
- `NULL IS NULL` → TRUE
- `NULL AND FALSE` → FALSE (false dominates)
- `NULL OR TRUE` → TRUE (true dominates)
- `NOT NULL` → NULL

## Error handling

```elixir
# Parse errors return {:error, message}
{:error, msg} = SqlExecutionEngine.execute("INVALID SQL", source)

# Runtime errors raise typed exceptions (wrapped in {:error, msg} by execute/2)
SqlExecutionEngine.execute("SELECT * FROM unknown_table", source)
# => {:error, "Table not found: unknown_table"}
```

Direct exception types (when calling Executor directly):
- `TableNotFoundError` — unknown table
- `ColumnNotFoundError` — unknown column reference
- `UnsupportedQueryError` — non-SELECT statement (INSERT, UPDATE, etc.)

## Running tests

```sh
cd code/packages/elixir/sql_execution_engine
mix deps.get
mix test
```
