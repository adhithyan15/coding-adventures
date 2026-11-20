/**
 * The v2 conformance kit, run by this lane (VER04r section 11; S7a).
 *
 * Every `v2-*` case whose `requires` is inside the lane's levels runs; per
 * category, run plus skipped equals the index count; `also_sequential` chunk
 * cases are also walked from chunk 0, and chunk `c` alone equals the walk.
 */
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { KNOWN_CATEGORIES, jsonEqual, runKit } from "../../src/v2/kit.ts";
import { KIT_DIR, kitWithExtraCategory, tamperedKit } from "./support.ts";

const index = JSON.parse(readFileSync(join(KIT_DIR, "index.json"), "utf8")) as { files: { category: string; cases: number }[]; total_cases: number };
const indexed = (category: string): number => index.files.find((row) => row.category === category)?.cases ?? 0;

describe("the committed kit", () => {
  const report = runKit(KIT_DIR, { sequential: true });

  it("passes every case of the six known categories: 0 failed, 0 skipped", () => {
    expect(report.failed).toBe(0);
    for (const category of KNOWN_CATEGORIES) {
      const row = report.categories.find((r) => r.category === category);
      expect(row, category).toBeDefined();
      expect(row?.failed, category).toEqual([]);
      expect(row?.skipped, category).toBe(0);
      expect(row?.run, category).toBe(indexed(category));
      expect((row?.run ?? 0) + (row?.skipped ?? 0), category).toBe(indexed(category));
      expect(row?.passed, category).toBe(indexed(category));
    }
  });

  it("holds at least the 782 cases S5b and S5e committed", () => {
    const known = KNOWN_CATEGORIES.reduce((sum, category) => sum + indexed(category), 0);
    expect(known).toBeGreaterThanOrEqual(782);
    expect(report.run).toBe(known);
  });

  it("treats a category it does not know (v2-binding) as digest-checked and skipped whole", () => {
    const unknown = index.files.filter((row) => row.category.startsWith("v2-") && !(KNOWN_CATEGORIES as readonly string[]).includes(row.category));
    expect(report.skipped).toBe(unknown.reduce((sum, row) => sum + row.cases, 0));
  });
});

describe("the harness", () => {
  it("skips a category it does not know and still counts it", () => {
    const dir = kitWithExtraCategory("v2-binding-x", 3);
    const report = runKit(dir, { sequential: false });
    const extra = report.categories.find((row) => row.category === "v2-binding-x");
    expect(extra).toMatchObject({ indexed: 3, run: 0, skipped: 3, passed: 0, failed: [], unrunnable: 3 });
    expect(report.failed).toBe(0);
    // cases of an unknown category the runner implements the levels of are cases nothing ran: a manifest may not be written (S7b)
    expect(report.unrunnable).toBe(3);
    const needsMore = runKit(kitWithExtraCategory("v2-binding-x", 2, ["ir.core", "ir.fixed"]), { sequential: false });
    expect(needsMore.categories.find((row) => row.category === "v2-binding-x")?.unrunnable).toBe(0);
    expect(needsMore.unrunnable).toBe(0);
  });

  it("skips a case whose requires names a level the runner lacks", () => {
    const dir = tamperedKit("v2-arith", (cases) => {
      (cases[0] as { requires: string[] }).requires = ["ir.core", "ir.fixed"];
    });
    const arith = runKit(dir, { sequential: false, only: ["v2-arith"] }).categories.find((row) => row.category === "v2-arith");
    expect(arith?.skipped).toBe(1);
    expect(arith?.run).toBe(indexed("v2-arith") - 1);
    expect(arith?.failed).toEqual([]);
    // A runner that implemented ir.fixed would run it.
    const wider = runKit(dir, { sequential: false, only: ["v2-arith"], levels: ["ir.core", "ir.fixed"] }).categories.find((row) => row.category === "v2-arith");
    expect(wider?.skipped).toBe(0);
  });

  it("reports a deliberately wrong expectation by case id and never echoes input", () => {
    const dir = tamperedKit("v2-arith", (cases) => {
      (cases[0] as { expect: { value: string } }).expect.value = "999";
    });
    const arith = runKit(dir, { sequential: false, only: ["v2-arith"] }).categories.find((row) => row.category === "v2-arith");
    expect(arith?.failed.map((f) => f.id)).toEqual(["v2-arith/u8-add-wrap"]);
    expect(arith?.failed[0]?.note).toMatch(/outcome differs/);
  });

  it("reports an error the runner does not map as a harness failure, not a crash", () => {
    const dir = tamperedKit("v2-value", (cases) => {
      (cases[0] as { input_hex: string }).input_hex = "zz";
    });
    const value = runKit(dir, { sequential: false, only: ["v2-value"] }).categories.find((row) => row.category === "v2-value");
    expect(value?.failed[0]?.note).toMatch(/harness error/);
  });

  it("refuses a fixture whose digest, count, total or file moved", () => {
    type Document = { files: Record<string, unknown>[]; total_cases: number };
    const edit = (change: (document: Document) => void): string => {
      const dir = tamperedKit("v2-arith", () => undefined);
      const document = JSON.parse(readFileSync(join(dir, "index.json"), "utf8"));
      change(document);
      writeFileSync(join(dir, "index.json"), JSON.stringify(document));
      return dir;
    };
    const row = (d: Document): Record<string, unknown> => d.files.find((f) => f["category"] === "v2-arith") as Record<string, unknown>;
    expect(() => runKit(edit((d) => { row(d)["sha256"] = "0".repeat(64); }), { sequential: false })).toThrow(/sha256 moved/);
    expect(() => runKit(edit((d) => { row(d)["cases"] = 1; d.total_cases = 1; }), { sequential: false })).toThrow();
    expect(() => runKit(edit((d) => { d.total_cases += 1; }), { sequential: false })).toThrow(/total_cases/);
    expect(() => runKit(edit((d) => { row(d)["path"] = "nope.json"; }), { sequential: false })).toThrow(/file missing/);
  });

  it("reports a chunk file that is consistent but is not what the enumeration gives", () => {
    const dir = tamperedKit("v2-chunk", (cases) => {
      const c = cases.find((x: { id: string }) => x.id === "v2-chunk/ragged-short-last-chunk") as Record<string, any>; // eslint-disable-line @typescript-eslint/no-explicit-any
      // an expected expression of cost 1 instead of 4: same row, different expected stream
      c["suite"]["oracle"]["expected"] = ["lit", { type: "Nat", value: "0" }];
      c["suite"]["ir_cost"] -= 3;
      c["expect"] = { error: "chunk-file" };
    });
    const chunk = runKit(dir, { sequential: false, only: ["v2-chunk"] }).categories.find((row) => row.category === "v2-chunk");
    expect(chunk?.failed).toEqual([]);
  });

  it("reports a value case whose type needs a level this runner lacks as a harness error", () => {
    const dir = tamperedKit("v2-value", (cases) => {
      Object.assign(cases[0] as object, { type: "(U8,U8)", input_hex: "5b5d" });
    });
    const value = runKit(dir, { sequential: false, only: ["v2-value"] }).categories.find((row) => row.category === "v2-value");
    expect(value?.failed[0]?.note).toMatch(/harness error/);
  });

  it("walks an also_sequential chunk case from chunk 0 unless asked not to", () => {
    const dir = tamperedKit("v2-chunk", (cases) => {
      for (const c of cases) if (c.id === "v2-chunk/ragged-short-last-chunk") expect(c.also_sequential).toBe(true);
    });
    expect(runKit(dir, { sequential: true, only: ["v2-chunk"] }).failed).toBe(0);
    expect(runKit(dir, { sequential: false, only: ["v2-chunk"] }).failed).toBe(0);
  });

  it("compares parsed JSON structurally", () => {
    expect(jsonEqual({ a: [1, { b: 2 }] }, { a: [1, { b: 2 }] })).toBe(true);
    expect(jsonEqual({ a: 1 }, { a: 2 })).toBe(false);
    expect(jsonEqual({ a: 1 }, { a: 1, b: 2 })).toBe(false);
    expect(jsonEqual([1, 2], [1])).toBe(false);
    expect(jsonEqual([1], { 0: 1 })).toBe(false);
    expect(jsonEqual("x", ["x"])).toBe(false);
  });
});
