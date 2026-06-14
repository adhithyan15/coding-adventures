/**
 * # The FormulaAdapter — the pluggability seam
 *
 * This interface is the single most important design decision in the package.
 * It is what makes the engine **generic**: the recalc core knows about cells, a
 * dependency graph, and topological evaluation — but it knows *nothing* about
 * Excel, about the `=` sign, or about what `SUM` means. All of that domain
 * knowledge is supplied from the outside via a `FormulaAdapter`.
 *
 * Think of it as a strategy/plugin boundary:
 *
 * ```text
 *     ┌──────────────────────────────────────────┐
 *     │   Workbook / recalc engine (generic)      │
 *     │   - stores cells                          │
 *     │   - builds the dependency graph           │
 *     │   - topologically orders + evaluates      │
 *     └───────────────────┬──────────────────────┘
 *                         │  asks the adapter:
 *                         │   "is this a formula?"
 *                         │   "what does it depend on?"
 *                         │   "evaluate it (here's a resolver)"
 *                         ▼
 *     ┌──────────────────────────────────────────┐
 *     │   FormulaAdapter (domain-specific)        │
 *     │   e.g. excelCasAdapter, or a toy adapter, │
 *     │   or something that isn't a spreadsheet   │
 *     │   at all (any table of related values).   │
 *     └──────────────────────────────────────────┘
 * ```
 *
 * Because the engine only ever talks to the adapter through these three
 * methods, you can drive the *exact same* recalc machinery with a completely
 * different formula language — or with no "language" at all, just a rule that
 * says "this cell is the sum of those two". The default Excel/CAS adapter lives
 * in `src/adapters/excel-cas.ts`; the core never imports it.
 */

import type { CellAddress } from "./address.js";
import type { CellValue } from "./cell-value.js";

/** A function the engine hands to the adapter at evaluation time. Given a cell
 *  address, it returns that cell's *current* value (already computed, because
 *  the engine evaluates in topological order). Empty/unknown cells come back as
 *  `{kind:"empty"}`; the adapter decides what empty coerces to. */
export type CellResolver = (addr: CellAddress) => CellValue;

export interface FormulaAdapter {
  /**
   * Is this raw cell content a formula (rather than a literal)?
   *
   * For the Excel adapter this is simply "does it start with `=`". A different
   * adapter might use a different sigil, or treat everything as a formula. When
   * this returns `false`, the engine parses the raw content as a literal
   * (numeric string → number, otherwise text) and never calls `dependencies`
   * or `evaluate` for that cell.
   */
  isFormula(raw: string): boolean;

  /**
   * Which cells does this formula reference?
   *
   * The engine uses the returned addresses to build the dependency graph:
   * an edge from this cell to each address. Ranges must be *expanded* to their
   * individual cells here (e.g. `SUM(A1:A3)` returns A1, A2, A3) so the graph
   * sees per-cell edges. Only called when `isFormula(raw)` is true.
   *
   * If the formula can't be parsed, return `[]` — the engine will still call
   * `evaluate`, which is where the adapter surfaces the parse error as a value.
   */
  dependencies(raw: string): CellAddress[];

  /**
   * Evaluate the formula to a single `CellValue`.
   *
   * `resolve` looks up the current value of any referenced cell. The engine
   * guarantees that, in the absence of cycles, every dependency has already
   * been evaluated by the time this is called (topological order). The adapter
   * should never throw for ordinary spreadsheet errors — it should return an
   * `{kind:"error", code}` value so the error propagates like Excel's do.
   */
  evaluate(raw: string, resolve: CellResolver): CellValue;
}
