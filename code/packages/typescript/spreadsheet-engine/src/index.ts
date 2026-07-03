/**
 * # @coding-adventures/spreadsheet-engine
 *
 * A headless spreadsheet computation core. The generic engine owns cells, a
 * dependency graph, and incremental topological recalc; **all formula knowledge
 * is pluggable** via a `FormulaAdapter`. A default Excel/CAS adapter ships so it
 * computes real formulas out of the box.
 *
 * ## Quick start
 *
 * ```ts
 * import { createSpreadsheet } from "@coding-adventures/spreadsheet-engine";
 *
 * const wb = createSpreadsheet();           // wired with the Excel/CAS adapter
 * wb.setCell("A1", "10");
 * wb.setCell("A2", "20");
 * wb.setCell("A3", "=SUM(A1:A2)");
 * wb.getValue("A3");                         // → { kind: "number", value: 30 }
 * wb.setCell("A1", "100");                   // auto-recalc downstream
 * wb.getValue("A3");                         // → { kind: "number", value: 120 }
 * ```
 *
 * ## Using your own adapter (the generic path)
 *
 * ```ts
 * import { Workbook, type FormulaAdapter } from "@coding-adventures/spreadsheet-engine";
 * const myAdapter: FormulaAdapter = { isFormula, dependencies, evaluate };
 * const wb = new Workbook({ adapter: myAdapter });
 * ```
 */

// --- The generic engine -----------------------------------------------------
export { Workbook } from "./workbook.js";
export type { RecalcMode, WorkbookOptions } from "./workbook.js";

// --- The pluggability seam --------------------------------------------------
export type { FormulaAdapter, CellResolver } from "./adapter.js";

// --- The value model --------------------------------------------------------
export type { CellValue, CellErrorCode } from "./cell-value.js";
export {
  EMPTY,
  num,
  text,
  bool,
  err,
  isError,
  toNumber,
  toText,
  toBoolean,
  formatValue,
} from "./cell-value.js";

// --- Addresses & ranges -----------------------------------------------------
export type { CellAddress, CellRange } from "./address.js";
export {
  parseA1,
  printA1,
  addressKey,
  columnToLetters,
  lettersToColumn,
  parseRange,
  normalizeRange,
  expandRange,
} from "./address.js";

// --- Cells ------------------------------------------------------------------
export type { Cell, LiteralCell, FormulaCell } from "./cell.js";

// --- The dependency graph (exported for advanced/inspection use) ------------
export { DependencyGraph } from "./dependency-graph.js";

// --- The default Excel/CAS adapter ------------------------------------------
export { excelCasAdapter } from "./adapters/excel-cas.js";

import { Workbook } from "./workbook.js";
import type { RecalcMode } from "./workbook.js";
import { excelCasAdapter } from "./adapters/excel-cas.js";

/**
 * Convenience constructor: a `Workbook` pre-wired with the default Excel/CAS
 * adapter. Equivalent to `new Workbook({ adapter: excelCasAdapter, mode })`.
 */
export function createSpreadsheet(options: { mode?: RecalcMode } = {}): Workbook {
  return new Workbook({ adapter: excelCasAdapter, mode: options.mode });
}
