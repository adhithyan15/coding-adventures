/**
 * The v2 conformance kit, run by this lane (VER04r section 11; S7a).
 *
 * Every `v2-*` case whose `requires` is inside the lane's levels runs; per
 * category, run plus skipped equals the index count; `also_sequential` chunk
 * cases are also walked from chunk 0, and chunk `c` alone equals the walk.
 */
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { beforeAll, describe, expect, it } from "vitest";
import { KNOWN_CATEGORIES, jsonEqual, runKit } from "../../src/v2/kit.ts";
import { KIT_DIR, kitWithExtraCategory, tamperedKit } from "./support.ts";

const index = JSON.parse(readFileSync(join(KIT_DIR, "index.json"), "utf8")) as { files: { category: string; cases: number }[]; total_cases: number };
const indexed = (category: string): number => index.files.find((row) => row.category === category)?.cases ?? 0;
// S10d adds 461 cases at ir.fixed and ir.tuple; 400 of them are skipped by their requires at this lane's levels (ir.core).
const SKIPPED: Record<string, number> = { "v2-value": 39, "v2-arith": 0, "v2-ir": 246, "v2-domain": 43, "v2-chunk": 15, "v2-fault": 57 };
const SKIPPED_TOTAL = Object.values(SKIPPED).reduce((sum, n) => sum + n, 0);

describe("the committed kit", () => {
  // The sequential walk of the three 2^24-case chunk cases is the slow part (about 5 s locally; CI is several times slower).
  let report = {} as ReturnType<typeof runKit>;
  beforeAll(() => {
    report = runKit(KIT_DIR, { sequential: true });
  }, 600_000);

  it("passes every case of the six known categories whose requires it implements: 0 failed", () => {
    expect(report.failed).toBe(0);
    for (const category of KNOWN_CATEGORIES) {
      const row = report.categories.find((r) => r.category === category);
      expect(row, category).toBeDefined();
      expect(row?.failed, category).toEqual([]);
      expect(row?.skipped, category).toBe(SKIPPED[category]);
      expect(row?.run, category).toBe(indexed(category) - SKIPPED[category]);
      expect((row?.run ?? 0) + (row?.skipped ?? 0), category).toBe(indexed(category));
      expect(row?.passed, category).toBe(indexed(category) - SKIPPED[category]);
    }
  });

  it("holds at least the 782 cases S5b and S5e committed", () => {
    const known = KNOWN_CATEGORIES.reduce((sum, category) => sum + indexed(category), 0);
    expect(known).toBeGreaterThanOrEqual(782);
    expect(report.run).toBe(known - SKIPPED_TOTAL);
  });

  it("treats a category it does not know (v2-binding) as digest-checked and skipped whole", () => {
    const unknown = index.files.filter((row) => row.category.startsWith("v2-") && !(KNOWN_CATEGORIES as readonly string[]).includes(row.category));
    expect(report.skipped).toBe(unknown.reduce((sum, row) => sum + row.cases, 0) + SKIPPED_TOTAL);
  });
});

describe("the harness", () => {
  it("skips a category it does not know and still counts it", () => {
    const dir = kitWithExtraCategory("v2-binding-x", 3);
    const report = runKit(dir, { sequential: false });
    const extra = report.categories.find((row) => row.category === "v2-binding-x");
    expect(extra).toMatchObject({ indexed: 3, run: 0, skipped: 3, passed: 0, failed: [], unrunnable: 3 });
    expect(report.failed).toBe(0);
    // cases of an unknown category the runner implements the levels of are cases nothing ran: a manifest may not be written (S7b).
    // The committed kit has such a category of its own since S5d: `v2-binding` (176 `ir.core` cases), which this runner
    // digest-checks and skips whole until S7b runs it.  So the extra category adds its 3 to whatever the rest already holds.
    const binding = report.categories.find((row) => row.category === "v2-binding");
    if (binding) {
      expect(binding).toMatchObject({ run: 0, failed: [] });
      expect(binding.skipped).toBe(binding.indexed);
      expect(binding.unrunnable).toBe(binding.indexed);
    }
    const others = report.categories
      .filter((row) => row.category !== "v2-binding-x")
      .reduce((sum, row) => sum + row.unrunnable, 0);
    expect(report.unrunnable).toBe(others + 3);
    const needsMore = runKit(kitWithExtraCategory("v2-binding-x", 2, ["ir.core", "ir.fixed"]), { sequential: false });
    expect(needsMore.categories.find((row) => row.category === "v2-binding-x")?.unrunnable).toBe(0);
    expect(needsMore.unrunnable).toBe(others);
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

  it("reports a chunk file that is consistent but is not what the enumeration gives as values, never as chunk-file (section 11 v2-chunk (b), (c))", () => {
    const edit = (expectation: (c: Record<string, any>) => void) => // eslint-disable-line @typescript-eslint/no-explicit-any
      tamperedKit("v2-chunk", (cases) => {
        const c = cases.find((x: { id: string }) => x.id === "v2-chunk/ragged-short-last-chunk") as Record<string, any>; // eslint-disable-line @typescript-eslint/no-explicit-any
        // an expected expression of cost 1 instead of 4: same row and file, different expected stream.  The body is
        // `subject == expected`, so it names the new expected as well and its cost counts twice.
        const was = JSON.stringify(c["suite"]["oracle"]["expected"]);
        c["suite"]["oracle"]["expected"] = ["lit", { type: "Nat", value: "0" }];
        c["suite"]["body"] = c["suite"]["body"].map((side: unknown) => (JSON.stringify(side) === was ? ["lit", { type: "Nat", value: "0" }] : side));
        c["suite"]["ir_cost"] -= 6;
        expectation(c);
      });
    // The committed expectation is no longer what the lane computes: a failed case with a value outcome, not a chunk-file refusal.
    const dir = edit(() => undefined);
    const chunk = runKit(dir, { sequential: false, only: ["v2-chunk"] }).categories.find((row) => row.category === "v2-chunk");
    expect(chunk?.failed).toEqual([{ id: "v2-chunk/ragged-short-last-chunk", note: "outcome differs (a value)" }]);
    // A refusal of that case is not what happens: expecting `chunk-file` fails too.
    const asRefusal = edit((c) => { c["expect"] = { error: "chunk-file" }; });
    const refusal = runKit(asRefusal, { sequential: false, only: ["v2-chunk"] }).categories.find((row) => row.category === "v2-chunk");
    expect(refusal?.failed).toHaveLength(1);
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
