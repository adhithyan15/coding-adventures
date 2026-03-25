# sql-csv-source (TypeScript)

A thin adapter that connects `@coding-adventures/sql-execution-engine` to
CSV files on disk. Each `tablename.csv` in a directory is one queryable table.

## How it fits in the stack

```
csv files on disk
       │
       ▼
 CsvDataSource          ← this package
       │
       ▼
sql-execution-engine    ← runs SELECT queries
       │
       ▼
  QueryResult
```

## Quick start

```typescript
import { CsvDataSource } from "@coding-adventures/sql-csv-source";
import { execute } from "@coding-adventures/sql-execution-engine";

const source = new CsvDataSource("path/to/csv/dir");
const result = execute("SELECT name FROM employees WHERE active = true", source);
result.rows.forEach(row => console.log(row.name));
// Alice
// Bob
// Dave
```

## Type coercion

| CSV string  | TypeScript value |
|-------------|------------------|
| `""`        | `null` (SQL NULL)|
| `"true"`    | `true`           |
| `"false"`   | `false`          |
| `"42"`      | `42` (number)    |
| `"3.14"`    | `3.14` (number)  |
| `"hello"`   | `"hello"` (string)|

## Installation

```bash
npm install @coding-adventures/sql-csv-source
```

## Dependencies

- `@coding-adventures/csv-parser` — parses CSV text into row objects
- `@coding-adventures/sql-execution-engine` — executes SELECT queries
