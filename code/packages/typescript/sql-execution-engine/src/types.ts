/**
 * Core types for the SQL execution engine.
 *
 * SQL Value Representation
 * ------------------------
 *
 * SQL values map to TypeScript as follows:
 *
 *   SQL NULL     → null
 *   INTEGER      → number (integer)
 *   REAL/FLOAT   → number (floating point)
 *   TEXT/VARCHAR → string
 *   BOOLEAN      → boolean
 *
 * We use `null` for SQL NULL and `number | string | boolean` for
 * non-NULL values. The union type `SqlValue` captures all possibilities.
 */

/** A nullable SQL value. `null` represents SQL NULL. */
export type SqlValue = null | number | string | boolean;

/** A single result row: maps column name to SQL value. */
export type Row = Record<string, SqlValue>;

/**
 * The output of a successfully executed SELECT query.
 *
 * @property columns - Output column names in SELECT order (after AS aliases).
 * @property rows    - Result rows, each a `Record<string, SqlValue>`.
 */
export interface QueryResult {
  columns: string[];
  rows: Row[];
}
