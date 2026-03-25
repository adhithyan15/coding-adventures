/**
 * index.ts — public exports for the @coding-adventures/csv-parser package.
 *
 * Re-exports the two main parsing functions and the error class so that
 * consumers can import everything from the package root:
 *
 * ```typescript
 * import { parseCSV, parseCSVWithDelimiter, UnclosedQuoteError } from '@coding-adventures/csv-parser';
 * ```
 */

export { parseCSV, parseCSVWithDelimiter } from "./parser.js";
export { UnclosedQuoteError } from "./errors.js";
export type { CsvRow, ParseState } from "./types.js";
