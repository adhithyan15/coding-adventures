/**
 * # The Cell — a literal value or a formula
 *
 * Every populated cell is one of two things (spec §1):
 *
 *   - a **literal**: a value typed directly, like `42` or `hello`. Its value
 *     never changes on its own.
 *   - a **formula**: a recipe like `=A1+B1`. Its value is *derived* and must be
 *     recomputed whenever an input changes. We cache the last computed value and
 *     the epoch at which we computed it, so incremental recalc can cheaply tell
 *     whether the cache is still fresh.
 *
 * We model this as a discriminated union on `kind`.
 */

import type { CellValue } from "./cell-value.js";

/** A cell holding a value typed directly by the user. */
export interface LiteralCell {
  readonly kind: "literal";
  /** The original text the user typed (so the UI can show it verbatim). */
  readonly raw: string;
  /** The parsed value. */
  readonly value: CellValue;
}

/** A cell holding a formula. Its `value` is a cache filled in by recalc. */
export interface FormulaCell {
  readonly kind: "formula";
  /** The formula source, e.g. `"=SUM(A1:A3)"`. */
  readonly raw: string;
  /** Cached result of the last evaluation; `undefined` until first computed. */
  value: CellValue | undefined;
  /** Workbook epoch at which `value` was last computed (for incremental skip). */
  lastEvalEpoch: number;
}

export type Cell = LiteralCell | FormulaCell;
