/**
 * Error classes for the SQL execution engine.
 *
 * All errors inherit from ExecutionError so callers can catch the whole
 * family with a single `catch (e)` clause and check `e instanceof ExecutionError`.
 *
 * Error Hierarchy:
 *
 *     ExecutionError          (base)
 *     ├── TableNotFoundError  (unknown table name)
 *     └── ColumnNotFoundError (unknown column reference)
 */

/** Base class for all SQL execution engine errors. */
export class ExecutionError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "ExecutionError";
  }
}

/**
 * Raised when the DataSource does not recognize a table name.
 *
 * @example
 *   execute("SELECT * FROM nonexistent", source)
 *   // throws TableNotFoundError("nonexistent")
 */
export class TableNotFoundError extends ExecutionError {
  readonly tableName: string;

  constructor(tableName: string) {
    super(`Table not found: ${JSON.stringify(tableName)}`);
    this.name = "TableNotFoundError";
    this.tableName = tableName;
  }
}

/**
 * Raised when a column reference cannot be resolved.
 *
 * @example
 *   execute("SELECT fake_col FROM employees", source)
 *   // throws ColumnNotFoundError("fake_col")
 */
export class ColumnNotFoundError extends ExecutionError {
  readonly columnName: string;

  constructor(columnName: string) {
    super(`Column not found: ${JSON.stringify(columnName)}`);
    this.name = "ColumnNotFoundError";
    this.columnName = columnName;
  }
}
