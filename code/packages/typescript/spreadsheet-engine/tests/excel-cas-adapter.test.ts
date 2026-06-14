import { describe, it, expect } from "vitest";
import { excelCasAdapter } from "../src/adapters/excel-cas.js";
import { createSpreadsheet, Workbook } from "../src/index.js";
import { parseA1, printA1 } from "../src/address.js";
import { EMPTY, num, text, type CellValue } from "../src/cell-value.js";
import type { CellResolver } from "../src/adapter.js";

/** A resolver backed by a plain {A1: value} map, for adapter-only tests. */
function mapResolver(cells: Record<string, CellValue>): CellResolver {
  return (addr) => cells[printA1(addr)] ?? EMPTY;
}

describe("excelCasAdapter.isFormula", () => {
  it("recognizes a leading = as a formula", () => {
    expect(excelCasAdapter.isFormula("=A1")).toBe(true);
    expect(excelCasAdapter.isFormula("42")).toBe(false);
    expect(excelCasAdapter.isFormula("hello")).toBe(false);
  });
});

describe("excelCasAdapter.dependencies", () => {
  it("extracts single cell refs", () => {
    const deps = excelCasAdapter.dependencies("=A1+B2").map(printA1);
    expect(deps.sort()).toEqual(["A1", "B2"]);
  });

  it("expands a range into its cells", () => {
    const deps = excelCasAdapter.dependencies("=SUM(B1:B5)").map(printA1);
    expect(deps).toEqual(["B1", "B2", "B3", "B4", "B5"]);
  });

  it("handles multi-range and mixed args", () => {
    const deps = excelCasAdapter.dependencies("=SUM(A1:A2,C1)").map(printA1);
    expect(deps.sort()).toEqual(["A1", "A2", "C1"]);
  });

  it("returns [] for an unparseable formula rather than throwing", () => {
    expect(excelCasAdapter.dependencies("=((")).toEqual([]);
  });
});

describe("excelCasAdapter.evaluate — arithmetic & precedence", () => {
  const cells = {
    A1: num(2),
    A2: num(4),
    A3: num(6),
    B2: num(3),
  };
  const resolve = mapResolver(cells);
  const ev = (f: string) => excelCasAdapter.evaluate(f, resolve);

  it("respects * over + precedence (=A1+B2*3 → 2 + 3*3 = 11)", () => {
    expect(ev("=A1+B2*3")).toMatchObject({ kind: "number", value: 11 });
  });

  it("honours parentheses (=(A1+B2)*3 → 15)", () => {
    expect(ev("=(A1+B2)*3")).toMatchObject({ value: 15 });
  });

  it("evaluates exponentiation right-associatively (=2^3^2 → 512)", () => {
    expect(ev("=2^3^2")).toMatchObject({ value: 512 });
  });

  it("handles subtraction and unary minus", () => {
    expect(ev("=A2-A1")).toMatchObject({ value: 2 });
    expect(ev("=-A1")).toMatchObject({ value: -2 });
  });

  it("handles floats (=1.5+2 → 3.5)", () => {
    expect(ev("=1.5+2")).toMatchObject({ value: 3.5 });
  });

  it("handles exact rational division (=10/4 → 2.5)", () => {
    expect(ev("=10/4")).toMatchObject({ value: 2.5 });
  });

  it("postfix percent (=A1% → 0.02)", () => {
    expect(ev("=A1%")).toMatchObject({ value: 0.02 });
  });

  it("text concatenation with & ", () => {
    const r = mapResolver({ A1: text("foo") });
    expect(excelCasAdapter.evaluate('=A1&"bar"', r)).toMatchObject({
      kind: "text",
      value: "foobar",
    });
  });

  it("comparison yields a boolean", () => {
    expect(ev("=A1<A2")).toMatchObject({ kind: "boolean", value: true });
    expect(ev("=A1=A2")).toMatchObject({ kind: "boolean", value: false });
  });
});

describe("excelCasAdapter.evaluate — functions", () => {
  const cells = { A1: num(1), A2: num(2), A3: num(3), B1: num(10), B5: num(50) };
  const resolve = mapResolver(cells);
  const ev = (f: string) => excelCasAdapter.evaluate(f, resolve);

  it("SUM over a range", () => {
    expect(ev("=SUM(A1:A3)")).toMatchObject({ value: 6 });
  });

  it("AVERAGE over a range", () => {
    expect(ev("=AVERAGE(A1:A3)")).toMatchObject({ value: 2 });
  });

  it("MIN / MAX / COUNT", () => {
    expect(ev("=MIN(A1:A3)")).toMatchObject({ value: 1 });
    expect(ev("=MAX(A1:A3)")).toMatchObject({ value: 3 });
    expect(ev("=COUNT(A1:A3)")).toMatchObject({ value: 3 });
  });

  it("SUM with mixed cell + literal args", () => {
    expect(ev("=SUM(A1,B1,5)")).toMatchObject({ value: 16 });
  });

  it("nested arithmetic on a function (=SUM(A1:A3)/2 → 3)", () => {
    expect(ev("=SUM(A1:A3)/2")).toMatchObject({ value: 3 });
  });

  it("AVERAGE over an empty range is #DIV/0!", () => {
    const r = mapResolver({});
    expect(excelCasAdapter.evaluate("=AVERAGE(Z1:Z3)", r)).toMatchObject({ code: "#DIV/0!" });
  });

  it("skips empty cells inside a range (blank ≠ zero for AVERAGE)", () => {
    // A1=1, A3=3, A2 blank → AVERAGE = (1+3)/2 = 2, not (1+0+3)/3
    const r = mapResolver({ A1: num(1), A3: num(3) });
    expect(excelCasAdapter.evaluate("=AVERAGE(A1:A3)", r)).toMatchObject({ value: 2 });
    expect(excelCasAdapter.evaluate("=COUNT(A1:A3)", r)).toMatchObject({ value: 2 });
  });

  it("unknown function → #NAME?", () => {
    expect(ev("=BOGUS(A1)")).toMatchObject({ code: "#NAME?" });
  });
});

describe("excelCasAdapter.evaluate — errors & edge cases", () => {
  const resolve = mapResolver({ A1: num(5) });

  it("division by zero → #DIV/0!", () => {
    expect(excelCasAdapter.evaluate("=1/0", resolve)).toMatchObject({ code: "#DIV/0!" });
    expect(excelCasAdapter.evaluate("=A1/0", resolve)).toMatchObject({ code: "#DIV/0!" });
  });

  it("empty cell coerces to 0 in arithmetic", () => {
    expect(excelCasAdapter.evaluate("=Z9+5", resolve)).toMatchObject({ value: 5 });
  });

  it("a bare number literal formula", () => {
    expect(excelCasAdapter.evaluate("=42", resolve)).toMatchObject({ value: 42 });
  });

  it("a bare string literal formula", () => {
    expect(excelCasAdapter.evaluate('="hi"', resolve)).toMatchObject({
      kind: "text",
      value: "hi",
    });
  });

  it("TRUE/FALSE keywords", () => {
    expect(excelCasAdapter.evaluate("=TRUE", resolve)).toMatchObject({ value: true });
    expect(excelCasAdapter.evaluate("=FALSE", resolve)).toMatchObject({ value: false });
  });

  it("non-numeric text where a number is needed → #VALUE!", () => {
    const r = mapResolver({ A1: text("abc") });
    expect(excelCasAdapter.evaluate("=A1+1", r)).toMatchObject({ code: "#VALUE!" });
  });

  it("an unparseable formula → #NAME?", () => {
    expect(excelCasAdapter.evaluate("=((", resolve)).toMatchObject({ code: "#NAME?" });
  });
});

describe("excelCasAdapter.evaluate — more coverage", () => {
  const cells = { A1: num(2), A2: num(4), A3: num(6), B1: num(10) };
  const resolve = mapResolver(cells);
  const ev = (f: string) => excelCasAdapter.evaluate(f, resolve);

  it("PRODUCT multiplies a range", () => {
    expect(ev("=PRODUCT(A1:A3)")).toMatchObject({ value: 48 }); // 2*4*6
  });

  it("MIN/MAX of an all-empty range return 0", () => {
    const r = mapResolver({});
    expect(excelCasAdapter.evaluate("=MIN(Z1:Z3)", r)).toMatchObject({ value: 0 });
    expect(excelCasAdapter.evaluate("=MAX(Z1:Z3)", r)).toMatchObject({ value: 0 });
    expect(excelCasAdapter.evaluate("=COUNT(Z1:Z3)", r)).toMatchObject({ value: 0 });
  });

  it("percent inside a larger expression", () => {
    expect(ev("=A1+10%")).toMatchObject({ value: 2.1 }); // 2 + 0.1
  });

  it("float-heavy arithmetic forces the float evaluator", () => {
    expect(ev("=1.5*2.5+0.5")).toMatchObject({ value: 4.25 });
    expect(ev("=2.0^0.5")).toMatchObject({ value: Math.SQRT2 });
  });

  it("unary plus is a no-op", () => {
    expect(ev("=+A1")).toMatchObject({ value: 2 });
  });

  it("textual comparison when operands aren't both numbers", () => {
    const r = mapResolver({ A1: text("apple"), A2: text("banana") });
    expect(excelCasAdapter.evaluate("=A1<A2", r)).toMatchObject({ value: true });
    expect(excelCasAdapter.evaluate("=A1>=A2", r)).toMatchObject({ value: false });
    expect(excelCasAdapter.evaluate("=A1<>A2", r)).toMatchObject({ value: true });
  });

  it("concatenation propagates an error operand", () => {
    expect(ev('=(1/0)&"x"')).toMatchObject({ code: "#DIV/0!" });
  });

  it("comparison propagates an error operand", () => {
    expect(ev("=(1/0)>1")).toMatchObject({ code: "#DIV/0!" });
  });

  it("an error inside a SUM range propagates", () => {
    const r = mapResolver({ A1: num(1), A2: { kind: "error", code: "#NA" } });
    expect(excelCasAdapter.evaluate("=SUM(A1:A2)", r)).toMatchObject({ code: "#NA" });
  });

  it("a single-cell reference used as a scalar resolves the cell", () => {
    expect(ev("=A1")).toMatchObject({ value: 2 });
  });

  it("a multi-cell range used as a scalar is #VALUE!", () => {
    expect(ev("=A1:A3")).toMatchObject({ code: "#VALUE!" });
  });

  it("error operand short-circuits arithmetic", () => {
    expect(ev("=(1/0)+A1")).toMatchObject({ code: "#DIV/0!" });
  });

  it("nested function in arithmetic", () => {
    expect(ev("=SUM(A1:A2)+MAX(A1:A3)")).toMatchObject({ value: 12 }); // 6 + 6
  });
});

describe("end-to-end through the Workbook (default adapter)", () => {
  it("sums a column and updates on edit", () => {
    const wb = createSpreadsheet();
    wb.setCell("A1", "1");
    wb.setCell("A2", "2");
    wb.setCell("A3", "3");
    wb.setCell("A4", "4");
    wb.setCell("A5", "5");
    wb.setCell("A6", "=SUM(A1:A5)");
    expect(wb.getValue("A6")).toMatchObject({ value: 15 });

    wb.setCell("A1", "11"); // change an input
    expect(wb.getValue("A6")).toMatchObject({ value: 25 });
  });

  it("chains formulas and recalcs the whole chain", () => {
    const wb = createSpreadsheet();
    wb.setCell("A1", "10");
    wb.setCell("B1", "=A1*2"); // 20
    wb.setCell("C1", "=B1+5"); // 25
    expect(wb.getValue("C1")).toMatchObject({ value: 25 });
    wb.setCell("A1", "100");
    expect(wb.getValue("B1")).toMatchObject({ value: 200 });
    expect(wb.getValue("C1")).toMatchObject({ value: 205 });
  });

  it("detects a circular reference end-to-end", () => {
    const wb = createSpreadsheet();
    wb.setCell("A1", "=B1");
    wb.setCell("B1", "=A1");
    expect(wb.getValue("A1")).toMatchObject({ code: "#CIRC!" });
    expect(wb.getValue("B1")).toMatchObject({ code: "#CIRC!" });
  });

  it("can be constructed explicitly with the exported adapter", () => {
    const wb = new Workbook({ adapter: excelCasAdapter });
    wb.setCell("A1", "=2+3");
    expect(wb.getValue("A1")).toMatchObject({ value: 5 });
  });
});
