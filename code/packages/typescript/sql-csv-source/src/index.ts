/**
 * index.ts — public exports for @coding-adventures/sql-csv-source.
 *
 * Re-exports the CsvDataSource class so consumers can import from
 * the package root:
 *
 * ```typescript
 * import { CsvDataSource } from "@coding-adventures/sql-csv-source";
 * ```
 */

export { CsvDataSource } from "./csv-data-source.js";
export { coerce } from "./csv-data-source.js";
