/**
 * csv-data-source.ts
 * ──────────────────
 * Thin adapter that implements the DataSource interface from
 * @coding-adventures/sql-execution-engine using CSV files from disk.
 *
 * Design
 * ------
 * The adapter is intentionally minimal. All the complexity of SQL evaluation
 * (filtering, joining, aggregation, ordering) lives in the execution engine.
 * This adapter's only jobs are:
 *
 *   1. Map a tableName to a file path: `{dir}/{tableName}.csv`
 *   2. Parse the CSV text into row objects via parseCSV from csv-parser
 *   3. Coerce each string value to its natural TypeScript/SQL type
 *   4. Report missing tables as TableNotFoundError
 *
 * Directory layout assumed:
 *
 *   data/
 *     employees.csv
 *     departments.csv
 *
 * Query: "SELECT * FROM employees" → reads data/employees.csv
 *
 * Type Coercion
 * -------------
 * CSV is untyped — every field is a string. The engine needs typed values
 * (null, boolean, number, string) to evaluate expressions like:
 *   WHERE salary > 80000   (number comparison)
 *   WHERE active = true    (boolean comparison)
 *   WHERE dept_id IS NULL  (null check)
 *
 * Coercion rules (applied in order):
 *
 *   | CSV string  | TypeScript value  |
 *   |-------------|-------------------|
 *   | ""          | null (SQL NULL)   |
 *   | "true"      | true              |
 *   | "false"     | false             |
 *   | "42"        | 42 (number)       |
 *   | "3.14"      | 3.14 (number)     |
 *   | "hello"     | "hello" (string)  |
 *
 * Column Ordering
 * ---------------
 * parseCSV returns Record<string, string> objects. In modern V8 (Node 16+),
 * string-keyed object properties do preserve insertion order when keys are
 * not array-like integers. However, to be explicit and safe, schema() reads
 * the first line of the file directly and splits on comma — this is a direct
 * read of the header row with zero ambiguity.
 *
 * File I/O
 * --------
 * Tests run in Node.js via vitest. We use node:fs and node:path — standard
 * built-ins with no additional dependencies. readFileSync is synchronous
 * which is fine for an educational/batch query tool (no server-side concerns).
 */

import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { existsSync } from "node:fs";

import { parseCSV } from "@coding-adventures/csv-parser";
import type { DataSource } from "@coding-adventures/sql-execution-engine";
import {
  TableNotFoundError,
} from "@coding-adventures/sql-execution-engine";
import type { SqlValue, Row } from "@coding-adventures/sql-execution-engine";

/**
 * A DataSource backed by CSV files in a directory.
 *
 * Each `tableName.csv` file in `dir` is one queryable table.
 * Column names come from the CSV header row. Values are type-coerced
 * from strings to the most appropriate TypeScript/SQL type.
 *
 * @example
 *   const source = new CsvDataSource("data/");
 *   const result = execute("SELECT * FROM employees WHERE active = true", source);
 *   result.rows.forEach(row => console.log(row));
 */
export class CsvDataSource implements DataSource {
  /**
   * @param dir - Path to the directory containing `*.csv` files.
   *   May be relative (resolved from the current working directory)
   *   or absolute.
   */
  constructor(private readonly dir: string) {}

  /**
   * Return column names for `tableName` in header order.
   *
   * Reads the first line of the CSV file and splits on comma.
   * This is fast (no full parse) and preserves the exact header order.
   *
   * @param tableName - Bare table name (e.g. `"employees"`).
   * @returns An array of column name strings.
   * @throws TableNotFoundError if `tableName.csv` does not exist.
   */
  schema(tableName: string): string[] {
    const content = this.readFile(tableName);
    // Split on newline; take the first line (the header row).
    const firstLine = content.split("\n")[0]?.trim() ?? "";
    if (!firstLine) return [];
    // Split the header on commas → ordered column names.
    return firstLine.split(",").map((col) => col.trim());
  }

  /**
   * Return all data rows from `tableName` with type-coerced values.
   *
   * Uses `parseCSV` for full RFC 4180 support (quoted fields with
   * embedded commas, escaped double-quotes). Each string value is then
   * passed through `coerce()` to produce a `SqlValue`.
   *
   * @param tableName - Bare table name (e.g. `"employees"`).
   * @returns Array of row objects. Empty array if no data rows.
   * @throws TableNotFoundError if `tableName.csv` does not exist.
   */
  scan(tableName: string): Row[] {
    const content = this.readFile(tableName);
    // parseCSV returns Record<string, string>[] — all values are strings.
    const strRows = parseCSV(content);
    // Coerce each string value to its natural SQL type.
    return strRows.map((row) =>
      Object.fromEntries(
        Object.entries(row).map(([k, v]) => [k, coerce(v)])
      )
    );
  }

  /**
   * Read the CSV file for `tableName`.
   *
   * @private
   * @throws TableNotFoundError if the file does not exist.
   */
  private readFile(tableName: string): string {
    const path = join(this.dir, `${tableName}.csv`);
    if (!existsSync(path)) {
      throw new TableNotFoundError(tableName);
    }
    return readFileSync(path, "utf-8");
  }
}

/**
 * Coerce a CSV string value to the most appropriate SQL type.
 *
 * This is the heart of the type-system bridge between CSV (untyped strings)
 * and SQL (typed values). The function is exported so it can be tested
 * independently and reused if needed.
 *
 * Rules applied in priority order:
 *
 * 1. `""` → `null`   — empty field is SQL NULL
 * 2. `"true"` → `true`   — boolean literal
 * 3. `"false"` → `false`  — boolean literal
 * 4. Integer-like string → `number` (e.g. `"42"` → `42`)
 *    We check `String(asInt) === value` to reject `"42.0"` as an integer.
 * 5. Float-like string → `number` (e.g. `"3.14"` → `3.14`)
 * 6. Anything else → `string`
 *
 * Why check `String(asInt) === value` for integers?
 *   `parseInt("42.5", 10)` returns `42` — it ignores the decimal.
 *   By round-tripping: `String(42) === "42.5"` is false, so "42.5" falls
 *   through to the float branch correctly.
 *
 * @param value - A single CSV field string.
 * @returns The coerced SqlValue (null | boolean | number | string).
 */
export function coerce(value: string): SqlValue {
  // ── NULL ───────────────────────────────────────────────────────────────────
  if (value === "") return null;

  // ── Boolean ────────────────────────────────────────────────────────────────
  // Exact string match (CSV convention: lowercase "true" / "false").
  if (value === "true") return true;
  if (value === "false") return false;

  // ── Integer ────────────────────────────────────────────────────────────────
  // parseInt() can return a valid integer even for "3.14" (it parses up to
  // the first non-digit). We guard against that by round-tripping:
  // if String(parseInt("3.14", 10)) === "3.14" is false, skip this branch.
  const asInt = parseInt(value, 10);
  if (!isNaN(asInt) && String(asInt) === value) return asInt;

  // ── Float ──────────────────────────────────────────────────────────────────
  // parseFloat("3.14") → 3.14; parseFloat("hello") → NaN.
  const asFloat = parseFloat(value);
  if (!isNaN(asFloat)) return asFloat;

  // ── String fallthrough ─────────────────────────────────────────────────────
  return value;
}
