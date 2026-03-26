/**
 * SQL Execution Engine — execute SELECT queries against pluggable data sources.
 *
 * This package is the execution layer of the SQL stack:
 *
 *   sql-lexer  →  sql-parser  →  sql-execution-engine
 *                                       ↑
 *                                  (this package)
 *
 * Public API:
 *
 *   import { execute, executeAll, DataSource, QueryResult } from
 *     "@coding-adventures/sql-execution-engine";
 *
 * Quick Start:
 *
 *   class MySource implements DataSource {
 *     schema(tableName: string): string[] { ... }
 *     scan(tableName: string): Row[] { ... }
 *   }
 *
 *   const result = execute("SELECT * FROM users WHERE age > 18", new MySource());
 *   console.log(result.columns); // ["id", "name", "age"]
 *   console.log(result.rows);    // [{ id: 1, name: "Alice", age: 30 }]
 */

export { execute, executeAll } from "./engine.js";
export type { DataSource } from "./data-source.js";
export type { QueryResult, Row, SqlValue } from "./types.js";
export {
  ExecutionError,
  TableNotFoundError,
  ColumnNotFoundError,
} from "./errors.js";
