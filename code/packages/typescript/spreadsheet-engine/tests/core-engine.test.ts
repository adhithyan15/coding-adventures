import { describe, it, expect } from "vitest";
import { Workbook } from "../src/workbook.js";
import { DependencyGraph } from "../src/dependency-graph.js";
import type { FormulaAdapter, CellResolver } from "../src/adapter.js";
import { parseA1, printA1, addressKey } from "../src/address.js";
import { num, toNumber, type CellValue } from "../src/cell-value.js";

/**
 * A TINY toy adapter, deliberately *not* Excel, to prove the engine core is
 * generic. Formula syntax: a leading ":" then a "+"-separated list of A1 cell
 * refs, e.g. `:A1+A2`. The value is the sum of the referenced cells' numbers.
 *
 * This exercises the whole engine (dependency tracking, topological recalc,
 * incremental update, cycle detection) with zero Excel/CAS involvement.
 */
const toyAdapter: FormulaAdapter = {
  isFormula: (raw) => raw.startsWith(":"),
  dependencies: (raw) =>
    raw
      .slice(1)
      .split("+")
      .map((s) => s.trim())
      .filter((s) => s.length > 0)
      .map(parseA1),
  evaluate: (raw, resolve: CellResolver): CellValue => {
    const refs = raw
      .slice(1)
      .split("+")
      .map((s) => s.trim())
      .filter((s) => s.length > 0);
    let sum = 0;
    for (const r of refs) {
      const v = resolve(parseA1(r));
      const n = toNumber(v);
      if (typeof n !== "number") return n; // propagate error
      sum += n;
    }
    return num(sum);
  },
};

function makeToy(mode: "auto" | "manual" = "auto"): Workbook {
  return new Workbook({ adapter: toyAdapter, mode });
}

describe("core engine — literals", () => {
  it("parses numeric literals as numbers and others as text", () => {
    const wb = makeToy();
    wb.setCell("A1", "42");
    wb.setCell("A2", "hello");
    expect(wb.getValue("A1")).toMatchObject({ kind: "number", value: 42 });
    expect(wb.getValue("A2")).toMatchObject({ kind: "text", value: "hello" });
  });

  it("reads unknown cells as empty", () => {
    const wb = makeToy();
    expect(wb.getValue("Z9")).toMatchObject({ kind: "empty" });
  });

  it("getRaw returns the original source", () => {
    const wb = makeToy();
    wb.setCell("A1", ":B1+C1");
    expect(wb.getRaw("A1")).toBe(":B1+C1");
    expect(wb.getRaw("Z9")).toBe("");
  });

  it("empty string clears a cell", () => {
    const wb = makeToy();
    wb.setCell("A1", "5");
    wb.setCell("A1", "");
    expect(wb.getValue("A1")).toMatchObject({ kind: "empty" });
  });
});

describe("core engine — topological recalc over a chain", () => {
  it("evaluates a dependency chain in order regardless of insertion order", () => {
    const wb = makeToy();
    // A3 depends on A2 depends on A1, but we insert the formulas first.
    wb.setCell("A3", ":A2"); // = A2
    wb.setCell("A2", ":A1"); // = A1
    wb.setCell("A1", "10");
    expect(wb.getValue("A2")).toMatchObject({ value: 10 });
    expect(wb.getValue("A3")).toMatchObject({ value: 10 });
  });

  it("a multi-input formula sums its dependencies", () => {
    const wb = makeToy();
    wb.setCell("A1", "1");
    wb.setCell("A2", "2");
    wb.setCell("A3", "3");
    wb.setCell("B1", ":A1+A2+A3");
    expect(wb.getValue("B1")).toMatchObject({ value: 6 });
  });
});

describe("core engine — incremental update on upstream edit", () => {
  it("changing an upstream cell updates everything downstream", () => {
    const wb = makeToy();
    wb.setCell("A1", "10");
    wb.setCell("A2", ":A1"); // 10
    wb.setCell("A3", ":A2"); // 10
    expect(wb.getValue("A3")).toMatchObject({ value: 10 });

    wb.setCell("A1", "100"); // edit the root
    expect(wb.getValue("A2")).toMatchObject({ value: 100 });
    expect(wb.getValue("A3")).toMatchObject({ value: 100 });
  });

  it("does not disturb unrelated cells", () => {
    const wb = makeToy();
    wb.setCell("A1", "1");
    wb.setCell("B1", "999"); // independent
    wb.setCell("A2", ":A1");
    wb.setCell("A1", "5");
    expect(wb.getValue("A2")).toMatchObject({ value: 5 });
    expect(wb.getValue("B1")).toMatchObject({ value: 999 });
  });
});

describe("core engine — cycle detection → #CIRC!", () => {
  it("marks both cells of a 2-cycle as #CIRC!", () => {
    const wb = makeToy();
    wb.setCell("A1", ":A2");
    wb.setCell("A2", ":A1"); // closes the loop
    expect(wb.getValue("A1")).toMatchObject({ code: "#CIRC!" });
    expect(wb.getValue("A2")).toMatchObject({ code: "#CIRC!" });
  });

  it("marks a self-reference as #CIRC!", () => {
    const wb = makeToy();
    wb.setCell("A1", ":A1");
    expect(wb.getValue("A1")).toMatchObject({ code: "#CIRC!" });
  });

  it("a longer cycle (A→B→C→A) flags all three", () => {
    const wb = makeToy();
    wb.setCell("A1", ":B1");
    wb.setCell("B1", ":C1");
    wb.setCell("C1", ":A1");
    expect(wb.getValue("A1")).toMatchObject({ code: "#CIRC!" });
    expect(wb.getValue("B1")).toMatchObject({ code: "#CIRC!" });
    expect(wb.getValue("C1")).toMatchObject({ code: "#CIRC!" });
  });
});

describe("core engine — manual vs auto recalc", () => {
  it("manual mode defers computation until recalcAll", () => {
    const wb = makeToy("manual");
    wb.setCell("A1", "10");
    wb.setCell("A2", ":A1");
    // Not computed yet in manual mode.
    expect(wb.getValue("A2")).toMatchObject({ kind: "empty" });
    wb.recalcAll();
    expect(wb.getValue("A2")).toMatchObject({ value: 10 });
  });

  it("setMode toggles behaviour", () => {
    const wb = makeToy("manual");
    wb.setCell("A1", "3");
    wb.setCell("A2", ":A1");
    wb.setMode("auto");
    wb.recalcAll();
    expect(wb.getValue("A2")).toMatchObject({ value: 3 });
  });
});

describe("core engine — bulk set + getValues snapshot", () => {
  it("setCells loads many cells then recalcs once", () => {
    const wb = makeToy();
    wb.setCells({ A1: "1", A2: "2", A3: ":A1+A2" });
    expect(wb.getValue("A3")).toMatchObject({ value: 3 });
  });

  it("getValues returns an A1-keyed snapshot", () => {
    const wb = makeToy();
    wb.setCell("A1", "1");
    wb.setCell("B2", ":A1");
    const snap = wb.getValues();
    expect(snap.A1).toMatchObject({ value: 1 });
    expect(snap.B2).toMatchObject({ value: 1 });
  });
});

describe("DependencyGraph unit behaviour", () => {
  it("tracks out/in edges and computes the dirty set", () => {
    const g = new DependencyGraph();
    const A1 = parseA1("A1");
    const A2 = parseA1("A2");
    const A3 = parseA1("A3");
    g.setDependencies(A2, [A1]); // A2 reads A1
    g.setDependencies(A3, [A2]); // A3 reads A2
    const dirty = g.dirtySet([A1]);
    expect(dirty.has(addressKey(A1))).toBe(true);
    expect(dirty.has(addressKey(A2))).toBe(true);
    expect(dirty.has(addressKey(A3))).toBe(true);
  });

  it("topo-orders a subset with dependencies first", () => {
    const g = new DependencyGraph();
    const A1 = parseA1("A1");
    const A2 = parseA1("A2");
    g.setDependencies(A2, [A1]);
    const subset = g.dirtySet([A1]);
    const { order, cyclic } = g.topoOrderSubset(subset);
    expect(cyclic.size).toBe(0);
    expect(order.indexOf(addressKey(A1))).toBeLessThan(order.indexOf(addressKey(A2)));
  });

  it("reports cyclic cells", () => {
    const g = new DependencyGraph();
    const A1 = parseA1("A1");
    const A2 = parseA1("A2");
    g.setDependencies(A1, [A2]);
    g.setDependencies(A2, [A1]);
    const subset = g.dirtySet([A1]);
    const { cyclic } = g.topoOrderSubset(subset);
    expect(cyclic.has(addressKey(A1))).toBe(true);
    expect(cyclic.has(addressKey(A2))).toBe(true);
  });

  it("setDependencies tears down old edges when a formula changes", () => {
    const g = new DependencyGraph();
    const A1 = parseA1("A1");
    const A2 = parseA1("A2");
    const A3 = parseA1("A3");
    g.setDependencies(A3, [A1]); // A3 reads A1
    g.setDependencies(A3, [A2]); // now A3 reads A2 instead
    expect(g.dirtySet([A1]).has(addressKey(A3))).toBe(false);
    expect(g.dirtySet([A2]).has(addressKey(A3))).toBe(true);
  });

  it("removeCell drops out-edges", () => {
    const g = new DependencyGraph();
    const A1 = parseA1("A1");
    const A2 = parseA1("A2");
    g.setDependencies(A2, [A1]);
    g.removeCell(A2);
    expect(g.dirtySet([A1]).has(addressKey(A2))).toBe(false);
  });

  it("exposes dependenciesOf / dependentsOf", () => {
    const g = new DependencyGraph();
    const A1 = parseA1("A1");
    const A2 = parseA1("A2");
    g.setDependencies(A2, [A1]);
    expect([...g.dependenciesOf(addressKey(A2))]).toContain(addressKey(A1));
    expect([...g.dependentsOf(addressKey(A1))]).toContain(addressKey(A2));
    // unknown keys yield empty sets
    expect(g.dependenciesOf("99,99").size).toBe(0);
    expect(g.dependentsOf("99,99").size).toBe(0);
  });

  it("orders a diamond deterministically (two parents feeding one child)", () => {
    const g = new DependencyGraph();
    const A1 = parseA1("A1");
    const B1 = parseA1("B1");
    const C1 = parseA1("C1");
    g.setDependencies(C1, [A1, B1]); // C1 = A1 + B1
    const subset = g.dirtySet([A1, B1]);
    const { order, cyclic } = g.topoOrderSubset(subset);
    expect(cyclic.size).toBe(0);
    expect(order.indexOf(addressKey(C1))).toBe(order.length - 1);
  });
});

describe("core engine — clearing a cell recalcs downstream", () => {
  it("clearing an upstream cell makes the dependent see empty (=0)", () => {
    const wb = makeToy();
    wb.setCell("A1", "10");
    wb.setCell("A2", ":A1");
    expect(wb.getValue("A2")).toMatchObject({ value: 10 });
    wb.setCell("A1", ""); // clear
    expect(wb.getValue("A2")).toMatchObject({ value: 0 });
  });
});
