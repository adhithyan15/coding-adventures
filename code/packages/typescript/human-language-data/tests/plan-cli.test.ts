import { describe, expect, it, vi, afterEach } from "vitest";
import { cpSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { runCompletionPlan } from "../src/plan-cli.js";
import { defaultCurriculumRoot } from "../src/loader.js";

// `plan-cli` had no test file at all, so every one of the security review's dirty
// controls left the suite green. These pin the coupling the review found: the
// presence list, the coverage measurement, and what happens when they disagree.

const roots: string[] = [];
afterEach(() => {
  for (const root of roots.splice(0)) rmSync(root, { recursive: true, force: true });
  vi.restoreAllMocks();
});

function corpus(): string {
  const root = mkdtempSync(join(tmpdir(), "hl-plan-"));
  roots.push(root);
  cpSync(defaultCurriculumRoot(), root, { recursive: true });
  return root;
}

function run(root: string, headSize = 25): { code: number; out: string; err: string } {
  let out = "";
  let err = "";
  vi.spyOn(process.stdout, "write").mockImplementation((chunk) => ((out += chunk), true));
  vi.spyOn(process.stderr, "write").mockImplementation((chunk) => ((err += chunk), true));
  const code = runCompletionPlan(["--root", root, "--head", String(headSize)]);
  return { code, out, err };
}

function editInventory(root: string, name: string, edit: (doc: Record<string, unknown>) => void): void {
  const path = join(root, "core", name);
  const doc = JSON.parse(readFileSync(path, "utf8")) as Record<string, unknown>;
  edit(doc);
  writeFileSync(path, JSON.stringify(doc));
}

describe("the plan CLI", () => {
  it("leads French and German with their exam gap on a clean corpus", () => {
    // HL16 adds one assessment-contract item per track ahead of content proxy
    // work. Ask for the complete enumerable queue so this test continues to
    // assert that the measured French/German exam gaps survive behind that gate.
    const { code, out } = run(corpus(), 200);
    expect(code).toBe(0);
    expect(out).toMatch(/exam-point — french/);
    expect(out).toMatch(/exam-point — german/);
    // Spanish USED to teach 100% of the points enumerated, so it got no
    // exam-point item at all. Enumerating the functional, notional and
    // orthographic dimensions took it to 223/273, so it now has 50 uncovered
    // points and joins the queue. That is the planner behaving correctly: the
    // work did not appear, it was always there and was invisible while the
    // target list covered one of four content dimensions.
    expect(out).toMatch(/exam-point — spanish/);
    // Still PARTIAL, and therefore still "0 complete": the pronunciation half
    // of phonology-orthography has no A1-only boundary in the source, so that
    // single dimension keeps the whole inventory partial. This is the HL20 rule
    // doing its job — three complete dimensions do not add up to a complete
    // file, and the inventory item is not suppressed.
    expect(out).toMatch(/0 complete and 4 partial of 138/);
  }, 120_000);

  it("does not let an unreadable inventory look like an absent one", () => {
    // The HIGH finding. `listExamInventories` lists any file that parses;
    // `loadExamInventory` is stricter. A file in the gap was listed as PRESENT
    // (so no `exam-inventory` item) and threw on load (so no `exam-point` item) —
    // the track vanished from both families while the report asserted its
    // inventory existed, silently, at exit 0.
    const root = corpus();
    editInventory(root, "exam-inventory-french-a1.json", (doc) => {
      doc.points = [];
    });
    const { code, out, err } = run(root);
    expect(code).toBe(1);
    expect(err).toMatch(/french A1 inventory exists but could not be read/);
    // It comes BACK as an inventory to write, which is what the old comment
    // falsely claimed already happened.
    expect(out).toMatch(/138\s+exam-inventory/);
    expect(out).toMatch(/0 complete and 3 partial of 138/);
    expect(out).toMatch(/1 exist but could not be READ/);
  }, 120_000);

  it("refuses an inventory whose points lost their probe key", () => {
    // `covered` is `probe !== null && …`, and a missing key is `undefined`, which
    // is not `null` — so an inventory with every probe deleted reported 100% and
    // suppressed its own work item. It never reached the try/catch.
    const root = corpus();
    editInventory(root, "exam-inventory-french-a1.json", (doc) => {
      for (const point of doc.points as Record<string, unknown>[]) delete point.probe;
    });
    const { code, err } = run(root);
    expect(code).toBe(1);
    expect(err).toMatch(/has no usable probe/);
  }, 120_000);

  it("counts a duplicated inventory once", () => {
    // `listExamInventories` dedupes nothing, so two files declaring the same
    // (language, level) produced two items with the IDENTICAL id, a doubled
    // projection, and a shrunken inventory backlog.
    const root = corpus();
    cpSync(
      join(root, "core", "exam-inventory-french-a1.json"),
      join(root, "core", "exam-inventory-fr-a1.json"),
    );
    const { out } = run(root);
    // 103 -> 98: the French questions chapter covered five A1 points (HL-C229).
    // German A2 adds 51 source-bounded points; three already have exact atoms.
    // 146 -> 196: Spanish A1 contributed 0 uncovered points while it enumerated
    // grammar only. Enumerating its functional, notional and orthographic
    // dimensions added 50 points with no corresponding atom, and this total is
    // the sum across all four written inventories.
    // 196 -> 194: HL23 §10 authors `saber` (chapter 389) and maps `A1-F2-16`
    // and `A1-F2-17`, the two PCIC ability points, onto it. The backlog falls
    // because two points were CLOSED, not because an inventory shrank — the
    // Spanish denominator is still 273.
    // 194 -> 190. HL23 §13 maps four `Nociones evaluativas` points: A1-NG6-03, -08
    // and -10 are closed by the qualities rung's own lessons, and -09 needed no
    // authoring at all — its note claimed the corpus never introduces `saber`, which
    // stopped being true when chapter 389 authored it two slices earlier.
    expect(out).toMatch(/190 uncovered point\(s\) across 4 written/);
    expect(out).toMatch(/0 complete and 4 partial of 138/);
    expect(out).toMatch(/the other 20 track\(s\)/);
  }, 120_000);

  it("rejects a flag used as another flag's value", () => {
    expect(run(corpus()).code).toBe(0);
    let err = "";
    vi.spyOn(process.stderr, "write").mockImplementation((chunk) => ((err += chunk), true));
    expect(runCompletionPlan(["--root", "--format"])).toBe(2);
    expect(err).toMatch(/--root requires a value/);
  }, 120_000);
});
