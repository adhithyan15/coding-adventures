/**
 * # The Workbook — the engine that ties it all together
 *
 * `Workbook` is the public face of the package. It owns:
 *
 *   - the **cells** (a map from address-key to `Cell`),
 *   - the **dependency graph**,
 *   - an **epoch** counter (bumped each recalc, for incremental freshness), and
 *   - a **recalc mode** (`auto` recomputes on every edit; `manual` waits).
 *
 * It is deliberately **generic**: all formula knowledge is delegated to the
 * injected `FormulaAdapter` (see `adapter.ts`). The workbook never imports the
 * Excel adapter or any parser — swap the adapter and the same machinery drives a
 * different formula language, or a non-spreadsheet table of related values.
 *
 * ## The recalc algorithm (spec §6), in one breath
 *
 * When you `setCell(a1, raw)`:
 *
 * ```text
 *   1. Parse the cell: formula? → store source; literal? → parse the value.
 *   2. If a formula, ask the adapter for its dependencies and rewrite the
 *      cell's out-edges in the dependency graph.
 *   3. (auto mode) Compute the dirty set = {this cell} ∪ everything downstream.
 *   4. Topologically order the dirty set.
 *   5. Evaluate cells in that order, caching each result.
 *   6. Any cell tangled in a cycle → value becomes #CIRC!.
 * ```
 *
 * Because step 3 starts from the edited cell and only walks *downstream*, an
 * edit to one cell in a 10 000-cell book recomputes just that cell and its
 * dependents — this is what "incremental recalc" means.
 */

import type { CellAddress } from "./address.js";
import { addressKey, parseA1, printA1 } from "./address.js";
import type { Cell } from "./cell.js";
import type { CellValue } from "./cell-value.js";
import { EMPTY, err, num, text } from "./cell-value.js";
import type { CellResolver, FormulaAdapter } from "./adapter.js";
import { DependencyGraph } from "./dependency-graph.js";

/** When does the workbook recompute? */
export type RecalcMode = "auto" | "manual";

export interface WorkbookOptions {
  /** The pluggable formula parser+evaluator. Required — it is the whole point. */
  adapter: FormulaAdapter;
  /** `"auto"` (default) recalculates after every edit; `"manual"` waits for an
   *  explicit `recalcAll()` call. */
  mode?: RecalcMode;
}

export class Workbook {
  private readonly adapter: FormulaAdapter;
  private mode: RecalcMode;

  /** address-key → Cell. The grid itself. */
  private readonly cells = new Map<string, Cell>();
  private readonly graph = new DependencyGraph();

  /** Bumped on every recalc pass; stamped onto each cell we (re)evaluate. */
  private epoch = 0;

  constructor(options: WorkbookOptions) {
    this.adapter = options.adapter;
    this.mode = options.mode ?? "auto";
  }

  /** Switch recalc mode at runtime. Switching *to* auto does not trigger a
   *  recalc by itself — call `recalcAll()` if you want one. */
  setMode(mode: RecalcMode): void {
    this.mode = mode;
  }

  // -------------------------------------------------------------------------
  // Editing
  // -------------------------------------------------------------------------

  /**
   * Set the contents of a cell from raw text (`"42"`, `"hello"`, `"=A1+B1"`).
   *
   * Empty string clears the cell. In `auto` mode this triggers an incremental
   * recalc of the cell and everything downstream; in `manual` mode it only
   * records the edit and updates the graph (call `recalcAll()` to compute).
   */
  setCell(a1: string, raw: string): void {
    const addr = parseA1(a1);
    const key = addressKey(addr);

    if (raw === "") {
      this.cells.delete(key);
      this.graph.removeCell(addr);
      if (this.mode === "auto") this.recalcFrom([addr]);
      return;
    }

    if (this.adapter.isFormula(raw)) {
      // A formula cell. Register its dependencies in the graph up front so the
      // dirty-set walk can see them, then (auto) recompute.
      const deps = this.adapter.dependencies(raw);
      this.graph.setDependencies(addr, deps);
      this.cells.set(key, {
        kind: "formula",
        raw,
        value: undefined,
        lastEvalEpoch: -1,
      });
    } else {
      // A literal. It has no dependencies, so clear any it used to have.
      this.graph.setDependencies(addr, []);
      this.cells.set(key, {
        kind: "literal",
        raw,
        value: parseLiteral(raw),
      });
    }

    if (this.mode === "auto") this.recalcFrom([addr]);
  }

  /** Convenience: set many cells, deferring recalc until all are in. Useful for
   *  bulk-loading a grid without N intermediate recalcs. */
  setCells(entries: Record<string, string>): void {
    const wasAuto = this.mode === "auto";
    this.mode = "manual";
    const touched: CellAddress[] = [];
    for (const [a1, raw] of Object.entries(entries)) {
      this.setCell(a1, raw);
      touched.push(parseA1(a1));
    }
    if (wasAuto) {
      this.mode = "auto";
      this.recalcFrom(touched);
    }
  }

  // -------------------------------------------------------------------------
  // Reading
  // -------------------------------------------------------------------------

  /** The current value of a cell. Unknown / blank cells read as `{kind:"empty"}`. */
  getValue(a1: string): CellValue {
    return this.valueAt(parseA1(a1));
  }

  /** The raw source text of a cell (`"=A1+B1"` or `"42"`), or `""` if blank. */
  getRaw(a1: string): string {
    return this.cells.get(addressKey(parseA1(a1)))?.raw ?? "";
  }

  /** Snapshot every non-empty cell's value, keyed by canonical A1 string.
   *  Handy for assertions and for rendering the whole grid. */
  getValues(): Record<string, CellValue> {
    const out: Record<string, CellValue> = {};
    for (const [key, cell] of this.cells) {
      const [col, row] = key.split(",").map(Number);
      out[printA1({ col, row })] = cell.value ?? EMPTY;
    }
    return out;
  }

  // -------------------------------------------------------------------------
  // Recalc
  // -------------------------------------------------------------------------

  /** Recompute *every* formula in the workbook. Bumps the epoch. Use this after
   *  bulk edits in manual mode, or to force a clean pass. */
  recalcAll(): void {
    this.recalcFrom([...this.cells.keys()].map(keyToAddress));
  }

  /**
   * The recalc core. Given the seed cells that just changed, compute the dirty
   * set, order it, and evaluate. Cells caught in a cycle become `#CIRC!`.
   */
  private recalcFrom(seeds: CellAddress[]): void {
    this.epoch++;

    // 1. Dirty set = seeds plus everything transitively downstream of them.
    const dirty = this.graph.dirtySet(seeds);

    // We only need to *evaluate* the dirty cells that are formulas; literals
    // already hold their value. But the graph's dirty set is keyed by address,
    // and some keys may be blank cells (referenced but never set) — those just
    // resolve to empty and need no evaluation.
    const { order, cyclic } = this.graph.topoOrderSubset(dirty);

    // 2. Cells tangled in a cycle: stamp #CIRC! and move on. We do this first
    //    so that any non-cyclic cell that *reads* a cyclic one sees the error.
    for (const key of cyclic) {
      const cell = this.cells.get(key);
      if (cell && cell.kind === "formula") {
        cell.value = err("#CIRC!");
        cell.lastEvalEpoch = this.epoch;
      }
    }

    // 3. Evaluate the acyclic cells in dependency order.
    const resolve: CellResolver = (addr) => this.valueAt(addr);
    for (const key of order) {
      const cell = this.cells.get(key);
      if (!cell || cell.kind !== "formula") continue; // literal or blank: skip
      // Belt-and-suspenders: the adapter *contract* says `evaluate` never throws
      // for ordinary errors, but the workbook must survive a *misbehaving* one.
      // Cell content is untrusted host input — a pathological formula could, for
      // instance, blow the adapter's recursion stack with a `RangeError`. If any
      // throw escaped this loop it would crash the host's `setCell` call and
      // potentially leave the workbook half-recalculated. We stamp `#VALUE!` and
      // keep going so one bad cell can never take down the engine.
      try {
        cell.value = this.adapter.evaluate(cell.raw, resolve);
      } catch {
        cell.value = err("#VALUE!");
      }
      cell.lastEvalEpoch = this.epoch;
    }
  }

  /** Resolve a cell address to its current value, for the adapter's resolver. */
  private valueAt(addr: CellAddress): CellValue {
    const cell = this.cells.get(addressKey(addr));
    if (!cell) return EMPTY;
    return cell.value ?? EMPTY;
  }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Turn raw non-formula text into a literal value: a fully-numeric string
 *  becomes a number, everything else becomes text. (Booleans typed as the bare
 *  words TRUE/FALSE are left to adapters; a literal "TRUE" here stays text,
 *  matching the conservative reading that only formulas interpret keywords.) */
function parseLiteral(raw: string): CellValue {
  const trimmed = raw.trim();
  if (trimmed !== "") {
    const n = Number(trimmed);
    if (!Number.isNaN(n)) return num(n);
  }
  return text(raw);
}

function keyToAddress(key: string): CellAddress {
  const [col, row] = key.split(",").map(Number);
  return { col, row };
}
