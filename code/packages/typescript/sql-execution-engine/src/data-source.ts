/**
 * DataSource — the pluggable data interface.
 *
 * The SQL execution engine is decoupled from any particular storage via
 * the `DataSource` interface. Implement this interface to connect the
 * engine to any data store.
 *
 * @example
 *   class InMemorySource implements DataSource {
 *     schema(tableName: string): string[] {
 *       if (tableName === "users") return ["id", "name"];
 *       throw new TableNotFoundError(tableName);
 *     }
 *
 *     scan(tableName: string): Row[] {
 *       if (tableName === "users") return [
 *         { id: 1, name: "Alice" },
 *         { id: 2, name: "Bob" },
 *       ];
 *       throw new TableNotFoundError(tableName);
 *     }
 *   }
 */

import type { Row, SqlValue } from "./types.js";

/**
 * Interface for pluggable data providers.
 *
 * The engine calls `schema()` first to discover column names, then
 * `scan()` to fetch rows. All filtering and aggregation happens
 * in-memory after the rows are returned.
 */
export interface DataSource {
  /**
   * Return the column names for the given table.
   *
   * @param tableName - The bare table name (no schema prefix).
   * @returns An array of column name strings.
   * @throws TableNotFoundError if the table is unknown.
   */
  schema(tableName: string): string[];

  /**
   * Return all rows of a table as an array of plain objects.
   *
   * Each object maps column name → SQL value. Values may be:
   * - `null`    — SQL NULL
   * - `number`  — integer or floating point
   * - `string`  — text
   * - `boolean` — true/false
   *
   * @param tableName - The bare table name.
   * @returns An array of row objects. May be empty.
   * @throws TableNotFoundError if the table is unknown.
   */
  scan(tableName: string): Row[];
}
