# sql-execution-engine (TypeScript)

A **SELECT-only SQL execution engine** that executes parsed SQL queries against
any pluggable data source.

## Usage

```typescript
import { execute, DataSource, SqlValue, SqlPrimitive } from "@coding-adventures/sql-execution-engine";

class MySource implements DataSource {
  schema(tableName: string): string[] {
    if (tableName === "users") return ["id", "name", "age"];
    throw new TableNotFoundError(tableName);
  }

  scan(tableName: string): Record<string, SqlValue>[] {
    if (tableName === "users") return [
      { id: 1, name: "Alice", age: 30 },
      { id: 2, name: "Bob",   age: 25 },
    ];
    throw new TableNotFoundError(tableName);
  }
}

const result = execute("SELECT name FROM users WHERE age > 27", new MySource());
console.log(result.columns); // ["name"]
console.log(result.rows);    // [{ name: "Alice" }]
```
