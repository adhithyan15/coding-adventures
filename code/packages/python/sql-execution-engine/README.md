# sql-execution-engine (Python)

A **SELECT-only SQL execution engine** that executes parsed SQL queries against
any pluggable data source. This package sits at the top of the SQL stack:

```
sql-lexer  →  sql-parser  →  sql-execution-engine
```

## What it does

Given a SQL string and a `DataSource` object, the engine:

1. Parses the SQL into an AST via `sql-parser`.
2. Evaluates the query through a full relational pipeline:
   - **FROM** — scans the table via `DataSource.scan()`.
   - **JOIN** — supports INNER, LEFT, RIGHT, FULL OUTER, and CROSS joins.
   - **WHERE** — filters rows using a recursive expression evaluator.
   - **GROUP BY** — groups rows for aggregate queries.
   - **HAVING** — filters groups post-aggregation.
   - **SELECT** — projects columns, evaluates expressions, applies aliases.
   - **DISTINCT** — deduplicates rows.
   - **ORDER BY** — sorts rows ASC or DESC.
   - **LIMIT / OFFSET** — paginates results.
3. Returns a `QueryResult` with column names and rows.

## Usage

```python
from sql_execution_engine import execute, DataSource, QueryResult

class MySource(DataSource):
    def schema(self, table_name: str) -> list[str]:
        if table_name == "users":
            return ["id", "name", "age"]
        raise TableNotFoundError(table_name)

    def scan(self, table_name: str) -> list[dict]:
        if table_name == "users":
            return [
                {"id": 1, "name": "Alice", "age": 30},
                {"id": 2, "name": "Bob",   "age": 25},
            ]
        raise TableNotFoundError(table_name)

source = MySource()

result = execute("SELECT name FROM users WHERE age > 27", source)
print(result.columns)  # ["name"]
print(result.rows)     # [{"name": "Alice"}]
```

## Supported SQL features

| Feature              | Example |
|----------------------|---------|
| SELECT *             | `SELECT * FROM t` |
| Column projection    | `SELECT id, name FROM t` |
| Aliases              | `SELECT name AS n FROM t` |
| WHERE                | `WHERE salary > 80000` |
| IS NULL / IS NOT NULL | `WHERE dept_id IS NULL` |
| BETWEEN              | `WHERE age BETWEEN 18 AND 65` |
| IN                   | `WHERE id IN (1, 2, 3)` |
| LIKE                 | `WHERE name LIKE 'A%'` |
| AND / OR / NOT       | `WHERE a = 1 AND NOT b = 2` |
| INNER JOIN           | `FROM a INNER JOIN b ON a.id = b.aid` |
| LEFT JOIN            | `FROM a LEFT JOIN b ON a.id = b.aid` |
| GROUP BY             | `GROUP BY dept_id` |
| Aggregates           | `COUNT(*), SUM(salary), AVG(salary), MIN, MAX` |
| HAVING               | `HAVING COUNT(*) > 1` |
| ORDER BY             | `ORDER BY salary DESC` |
| LIMIT / OFFSET       | `LIMIT 10 OFFSET 5` |
| DISTINCT             | `SELECT DISTINCT dept_id FROM t` |
| Arithmetic           | `SELECT salary * 1.1 AS adjusted FROM t` |

## Architecture

```
engine.py        — public execute() / execute_all() entry points
executor.py      — full relational pipeline (FROM → ORDER BY → LIMIT)
expression.py    — recursive expression evaluator
aggregate.py     — COUNT, SUM, AVG, MIN, MAX
join.py          — five join types
result.py        — QueryResult dataclass
data_source.py   — DataSource ABC
errors.py        — ExecutionError, TableNotFoundError, ColumnNotFoundError
```
