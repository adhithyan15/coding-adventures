# sql-execution-engine (Swift)

A dependency-free, SELECT-only SQL execution engine for the coding-adventures
SQL stack. It evaluates SQL against any data source that implements
`SqlDataSource`.

```swift
import SqlExecutionEngine

let source = InMemoryDataSource()
    .addTable(
        "users",
        schema: ["id", "name", "age"],
        rows: [
            ["id": 1, "name": "Alice", "age": 30],
            ["id": 2, "name": "Bob", "age": 25],
        ]
    )

let result = try SqlEngine.execute(
    "SELECT name FROM users WHERE age > 27",
    dataSource: source
)
```

The engine supports projection, expressions, SQL NULL logic, INNER/LEFT/RIGHT/
FULL/CROSS joins, GROUP BY, HAVING, COUNT/SUM/AVG/MIN/MAX, ORDER BY, DISTINCT,
LIMIT, and OFFSET. Parsing is intentionally local to this package so the port
does not create a broken dependency edge while Swift's reusable grammar-driven
SQL lexer/parser pair remains a separate Priority 2 item.
