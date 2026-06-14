/**
 * # Resilience / availability tests — untrusted cell content must never crash
 *
 * Cell content is host-supplied and therefore *untrusted*. These tests pin two
 * availability guards that a security review flagged, both of which turn a
 * resource-exhaustion attack into an ordinary spreadsheet error value:
 *
 *   A. **Unbounded range expansion (OOM).** A formula like `=SUM(A1:ZZ1000000)`
 *      names ~700 million cells. `expandRange` used to materialize one object
 *      per cell with no cap, so the dependency scan (which runs on `setCell`,
 *      before evaluation) would try to allocate a 700M-element array and OOM the
 *      host. We now cap at `MAX_RANGE_CELLS` and degrade to `#REF!`.
 *
 *   B. **Uncaught `RangeError` (stack overflow).** A pathologically deep formula
 *      (`=1+1+1+…` with thousands of terms) overflows the recursive evaluator's
 *      call stack with a `RangeError`. The adapter only caught its own
 *      `FormulaError` and re-threw everything else, so the `RangeError` escaped
 *      `setCell` and crashed the host. We now degrade any non-`FormulaError`
 *      throw to `#VALUE!`, with a belt-and-suspenders guard in the workbook too.
 *
 * The bar for "without OOM/hang" is wall-clock: each test below also asserts it
 * completes in well under a second, which it cannot do if the giant array is
 * actually allocated.
 */

import { describe, it, expect } from "vitest";
import { excelCasAdapter } from "../src/adapters/excel-cas.js";
import { createSpreadsheet } from "../src/index.js";
import { Workbook } from "../src/workbook.js";
import {
  expandRange,
  parseRange,
  rangeCellCount,
  RangeTooLargeError,
  MAX_RANGE_CELLS,
} from "../src/address.js";
import { EMPTY, type CellValue } from "../src/cell-value.js";
import type { CellResolver } from "../src/adapter.js";

/** A resolver that treats every cell as empty — enough for these probes, since
 *  the formulas under test fail (or aggregate empties) long before any real
 *  cell value matters. */
const emptyResolver: CellResolver = () => EMPTY;

// ===========================================================================
// Fix A — unbounded range expansion (OOM)
// ===========================================================================

describe("Fix A — range expansion is capped (no OOM)", () => {
  it("MAX_RANGE_CELLS is the documented single-column ceiling (2^20)", () => {
    expect(MAX_RANGE_CELLS).toBe(1_048_576);
  });

  it("rangeCellCount counts corners without materializing", () => {
    // 702 columns (A..ZZ) × 1,000,000 rows ≈ 700 million cells.
    const range = parseRange("A1:ZZ1000000");
    const count = rangeCellCount(range);
    expect(count).toBeGreaterThan(700_000_000);
  });

  it("expandRange throws RangeTooLargeError on an oversized range, fast", () => {
    const range = parseRange("A1:ZZ1000000");
    const start = Date.now();
    expect(() => expandRange(range)).toThrow(RangeTooLargeError);
    // It must reject from the O(1) corner arithmetic, never partially fill.
    expect(Date.now() - start).toBeLessThan(100);
  });

  it("expandRange still materializes a range exactly at the cap", () => {
    // A 1 × MAX_RANGE_CELLS column is the largest allowed; one more row is not.
    const ok = parseRange(`A1:A${MAX_RANGE_CELLS}`);
    expect(rangeCellCount(ok)).toBe(MAX_RANGE_CELLS);
    expect(expandRange(ok)).toHaveLength(MAX_RANGE_CELLS);

    const tooBig = parseRange(`A1:A${MAX_RANGE_CELLS + 1}`);
    expect(() => expandRange(tooBig)).toThrow(RangeTooLargeError);
  });

  it("=SUM over a huge range returns #REF! quickly (no OOM/hang)", () => {
    const start = Date.now();
    const v = excelCasAdapter.evaluate("=SUM(A1:ZZ1000000)", emptyResolver);
    expect(v).toMatchObject({ kind: "error", code: "#REF!" });
    expect(Date.now() - start).toBeLessThan(500);
  });

  it("dependencies() of a huge range registers no edges and never allocates", () => {
    const start = Date.now();
    const deps = excelCasAdapter.dependencies("=SUM(A1:ZZ1000000)");
    expect(deps).toEqual([]);
    expect(Date.now() - start).toBeLessThan(500);
  });

  it("setCell with a huge range degrades to an error value, never throws/OOMs", () => {
    const wb = createSpreadsheet();
    const start = Date.now();
    expect(() => wb.setCell("A1", "=SUM(A1:ZZ1000000)")).not.toThrow();
    const v: CellValue = wb.getValue("A1");
    expect(v).toMatchObject({ kind: "error" });
    expect(v.kind === "error" && v.code).toBe("#REF!");
    expect(Date.now() - start).toBeLessThan(500);
  });

  it("an ordinary-sized range still works (cap doesn't break normal use)", () => {
    const wb = createSpreadsheet();
    wb.setCell("A1", "1");
    wb.setCell("A2", "2");
    wb.setCell("A3", "3");
    wb.setCell("A4", "=SUM(A1:A3)");
    expect(wb.getValue("A4")).toMatchObject({ value: 6 });
  });
});

// ===========================================================================
// Fix B — a pathologically deep formula must not throw out of the engine
// ===========================================================================

describe("Fix B — deep nesting degrades to an error, never throws", () => {
  /** `=1+1+1+…` with `n` terms. Deep enough (thousands) to overflow the
   *  recursive evaluator's call stack with a RangeError. */
  const deepFormula = (n: number) => "=" + Array(n).fill("1").join("+");

  it("adapter.evaluate returns an error CellValue and does not throw", () => {
    let threw = false;
    let v: CellValue | undefined;
    try {
      v = excelCasAdapter.evaluate(deepFormula(7000), emptyResolver);
    } catch {
      threw = true;
    }
    expect(threw).toBe(false);
    expect(v).toMatchObject({ kind: "error" });
  });

  it("setCell with a deep formula does NOT throw and stamps an error value", () => {
    const wb = createSpreadsheet();
    expect(() => wb.setCell("A1", deepFormula(7000))).not.toThrow();
    expect(wb.getValue("A1")).toMatchObject({ kind: "error" });
  });

  it("the engine survives a bad cell and keeps recalculating others", () => {
    const wb = createSpreadsheet();
    wb.setCell("A1", deepFormula(7000)); // pathological
    wb.setCell("B1", "=2+3"); // ordinary, set AFTER the bad one
    // The bad cell is an error, but the workbook is intact and still computes.
    expect(wb.getValue("A1")).toMatchObject({ kind: "error" });
    expect(wb.getValue("B1")).toMatchObject({ value: 5 });
  });

  it("a misbehaving adapter that throws can never crash the workbook", () => {
    // Directly exercise the workbook's belt-and-suspenders try/catch with an
    // adapter whose evaluate() always throws a non-FormulaError.
    const throwingAdapter = {
      isFormula: (raw: string) => raw.startsWith("="),
      dependencies: () => [],
      evaluate: () => {
        throw new RangeError("boom");
      },
    };
    const wb = new Workbook({ adapter: throwingAdapter });
    expect(() => wb.setCell("A1", "=anything")).not.toThrow();
    expect(wb.getValue("A1")).toMatchObject({ kind: "error", code: "#VALUE!" });
  });
});
