/**
 * types.ts — shared type definitions for the CSV parser.
 *
 * Keeping types in a dedicated file makes them easy to import from any module
 * in the package without creating circular dependencies.
 */

/**
 * A single parsed CSV row: a map from column name (string) to field value (string).
 *
 * All values are strings. The CSV format has no type system — everything is text.
 * Type coercion (e.g., turning "42" into the number 42) is the caller's responsibility.
 *
 * Example:
 * ```
 * // CSV row: Alice,30,New York
 * // After parsing with header [name, age, city]:
 * const row: CsvRow = { name: "Alice", age: "30", city: "New York" };
 * ```
 */
export type CsvRow = Record<string, string>;

/**
 * The four states of the CSV parser state machine.
 *
 * At any point during parsing, the machine is in exactly one of these states.
 * Each state determines how the next character will be interpreted.
 *
 * Think of states like "modes" you are in while reading the file:
 *
 * | State               | You are...                                         |
 * |---------------------|----------------------------------------------------|
 * | FIELD_START         | About to start reading a new field                 |
 * | IN_UNQUOTED_FIELD   | Reading plain text (commas terminate the field)    |
 * | IN_QUOTED_FIELD     | Inside "...", commas and newlines are literal      |
 * | IN_QUOTED_MAYBE_END | Just saw " inside a quoted field; waiting for next |
 */
export type ParseState =
  | "FIELD_START"
  | "IN_UNQUOTED_FIELD"
  | "IN_QUOTED_FIELD"
  | "IN_QUOTED_MAYBE_END";
