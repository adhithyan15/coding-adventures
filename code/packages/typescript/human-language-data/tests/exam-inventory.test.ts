// ---------------------------------------------------------------------------
// The gate that answers the owner's question (HL-C128).
//
//     "The goal is not whether something touches some level. The goal is can
//      someone pass that level of exam with just reading the book and slowly
//      following its gentle ramp."
//
// Every other measurement in this package walks our own lessons, so every one
// of them rises when a lesson is added — including a lesson on something the
// exam does not test. This one resolves the corpus against an external, finite
// list, so it can fall, and it can stay flat while the corpus grows.
// ---------------------------------------------------------------------------
import { describe, expect, it } from "vitest";
import { listExamInventories, loadExamInventory } from "../src/loader.js";
import {
  EXAM_CONTENT_DIMENSIONS,
  measureExamCoverage,
  formatExamCoverage,
  isExamInventoryComplete,
  trackIntroducedAtoms,
  type ExamInventory,
} from "../src/exam-inventory.js";
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { COMPLETE_SCOPE, FIXTURE, fixtureLesson as lesson } from "./exam-inventory-fixture.js";

describe("probe semantics", () => {
  const corpus = [lesson("ES-1", ["ES-A"]), lesson("ES-2", ["ES-B"])];

  it("covers a point only when EVERY atom of its probe is introduced", () => {
    const coverage = measureExamCoverage(FIXTURE, corpus);
    const byId = new Map(coverage.points.map((point) => [point.id, point]));

    expect(byId.get("P-1")!.covered).toBe(true);
    // Half a paradigm is not half a mark in an exam, so it is not half a point
    // here. P-2 holds ES-A and is still uncovered.
    expect(byId.get("P-2")!.covered).toBe(false);
    expect(byId.get("P-2")!.missingAtoms).toEqual(["ES-MISSING"]);
  });

  it("counts an unmapped point as UNCOVERED rather than skipping it", () => {
    // The tempting reading of `probe: null` is "not yet classified, exclude from
    // the denominator". That would let the percentage be improved by deleting
    // the mapping — the one edit that changes nothing about what a reader knows.
    const coverage = measureExamCoverage(FIXTURE, corpus);
    expect(coverage.enumerated).toBe(3);
    expect(coverage.points.find((point) => point.id === "P-3")!.covered).toBe(false);
    expect(coverage.unmapped).toBe(1);
    expect(coverage.covered).toBe(1);
  });

  it("falls when a probed atom stops being introduced", () => {
    // The property an annotation cannot have. Retire the lesson that introduces
    // ES-B and P-1 must stop counting, with no edit to the inventory.
    const before = measureExamCoverage(FIXTURE, corpus);
    const after = measureExamCoverage(FIXTURE, [lesson("ES-1", ["ES-A"])]);
    expect(before.covered).toBe(1);
    expect(after.covered).toBe(0);
  });

  it("reads atoms through the shared helper, so the flat dotted key is honoured", () => {
    // `introduces.knowledge` is a FLAT frontmatter key. Reading it as a nested
    // object returns undefined for every lesson in the corpus and would report
    // 0% coverage — which looks like a devastating finding rather than a bug.
    expect(trackIntroducedAtoms(corpus, "spanish")).toEqual(new Set(["ES-A", "ES-B"]));
    expect(trackIntroducedAtoms(corpus, "french").size).toBe(0);
  });
});

describe("inventory completeness", () => {
  it("requires every content dimension and keeps partial point coverage measurable", () => {
    expect(Object.keys(COMPLETE_SCOPE).sort()).toEqual([...EXAM_CONTENT_DIMENSIONS].sort());
    expect(isExamInventoryComplete(FIXTURE)).toBe(true);
    const partial = {
      ...FIXTURE,
      scope: {
        ...COMPLETE_SCOPE,
        lexicon: { ...COMPLETE_SCOPE.lexicon, status: "partial" as const },
      },
    };
    expect(isExamInventoryComplete(partial)).toBe(false);
    const coverage = measureExamCoverage(partial, [lesson("ES-1", ["ES-A", "ES-B"])]);
    expect(coverage).toMatchObject({ enumerated: 3, covered: 1, inventoryComplete: false });
    expect(formatExamCoverage(coverage)).toContain("(partial inventory)");
  });
});

describe("the loader refuses what would move the number the wrong way", () => {
  it("refuses a language or level that could escape the curriculum root", () => {
    // `join` NORMALISES `..` inside an interpolated filename rather than
    // rejecting it, so before the guard `level = "../../../../etc/shadow"`
    // resolved to `/etc/shadow.json`. The trailing `.json` was no protection:
    // `.docker/config.json` holds registry credentials. Found by a security
    // review of this very PR, while the only callers still passed literals.
    expect(() => loadExamInventory("../../../../../../etc/passwd", "a1")).toThrow(/unsafe/);
    expect(() => loadExamInventory("spanish", "../../../../../../root/.docker/config")).toThrow(/unsafe/);
    expect(() => loadExamInventory("spanish", "A1/../A1")).toThrow(/unsafe/);
    // The control: the legitimate call still works, so the guard is not simply
    // refusing everything.
    expect(loadExamInventory("spanish", "A1").points.length).toBeGreaterThan(0);
  });
});

describe("a hostile inventory cannot corrupt the process", () => {
  function hostile(category: string): ExamInventory {
    return {
      version: 1,
      language: "spanish",
      level: "A1",
      about: "fixture",
      source: "fixture",
      scope: COMPLETE_SCOPE,
      probeSemantics: "fixture",
      points: [{ id: "X", category, label: "attack", probe: null }],
    };
  }

  it("does not write onto Object.prototype through a category name", () => {
    // With a plain `{}` accumulator, `byCategory["__proto__"] ??= {...}` finds
    // Object.prototype (truthy), skips the assignment, and `+= 1` lands on the
    // prototype itself — after which EVERY object in the process inherits
    // `enumerated: NaN`. The accumulator is `Object.create(null)` for this
    // reason, and this test is what stops that turning back into `{}`.
    const coverage = measureExamCoverage(hostile("__proto__"), []);
    expect(Object.prototype.hasOwnProperty.call(Object.prototype, "enumerated")).toBe(false);
    expect(({} as Record<string, unknown>).enumerated).toBeUndefined();

    // And the report stays self-consistent: the category is a real own key, so
    // the per-category lines still sum to the total. Under the old accumulator
    // the point vanished from `byCategory` while still counting in `enumerated`.
    expect(Object.keys(coverage.byCategory)).toEqual(["__proto__"]);
    const summed = Object.values(coverage.byCategory).reduce((total, entry) => total + entry.enumerated, 0);
    expect(summed).toBe(coverage.enumerated);
  });

  it("hands back a NORMAL object, so a consumer can still call hasOwnProperty", () => {
    // The first fix here was `Object.create(null)`, which closes the sink but
    // leaks into a public return type: `byCategory.hasOwnProperty(x)` and
    // `String(byCategory)` both throw on a null-prototype object. The Map plus
    // `Object.fromEntries` is safe for the same reason and normal to hold.
    const coverage = measureExamCoverage(hostile("__proto__"), []);
    expect(Object.getPrototypeOf(coverage.byCategory)).toBe(Object.prototype);
    expect(Object.prototype.hasOwnProperty.call(coverage.byCategory, "__proto__")).toBe(true);
    expect(() => JSON.stringify(coverage.byCategory)).not.toThrow();
    // And the safety survives the change: still an own key, still no pollution.
    expect(({} as Record<string, unknown>).enumerated).toBeUndefined();
  });

  it("reports 0% rather than NaN% for an inventory with no points", () => {
    const coverage = measureExamCoverage({ ...hostile("Uno"), points: [] }, []);
    expect(coverage.percent).toBe(0);
    expect(Number.isNaN(coverage.percent)).toBe(false);
  });
});

describe("the loader refuses a malformed file rather than crashing downstream", () => {
  // These exercise the SECONDARY defenses, which the primary tests above do not
  // reach: a caller cannot hand `loadExamInventory` a bad object, only a bad
  // file, so testing them needs a file.
  function withTempInventory<T>(body: (root: string) => T): T {
    const root = mkdtempSync(join(tmpdir(), "exam-inventory-"));
    mkdirSync(join(root, "core"));
    try {
      return body(root);
    } finally {
      rmSync(root, { recursive: true, force: true });
    }
  }
  const write = (root: string, value: unknown) =>
    writeFileSync(join(root, "core", "exam-inventory-es-a1.json"), JSON.stringify(value));

  it("names the file when `points` is missing or empty", () => {
    withTempInventory((root) => {
      write(root, { version: 1, language: "spanish", level: "A1" });
      expect(() => loadExamInventory("spanish", "A1", root)).toThrow(/no non-empty 'points' array/);
      write(root, { version: 1, points: [] });
      expect(() => loadExamInventory("spanish", "A1", root)).toThrow(/no non-empty 'points' array/);
    });
  });

  it("refuses a reserved category name before it reaches the accumulator", () => {
    withTempInventory((root) => {
      write(root, { ...FIXTURE, points: [{ id: "X", category: "__proto__", label: "l", probe: null }] });
      expect(() => loadExamInventory("spanish", "A1", root)).toThrow(/reserved category name/);
    });
  });

  it("refuses a duplicate point id, which would double-count a category", () => {
    withTempInventory((root) => {
      write(root, {
        ...FIXTURE,
        points: [
          { id: "X", category: "Uno", label: "l", probe: null },
          { id: "X", category: "Uno", label: "l", probe: null },
        ],
      });
      expect(() => loadExamInventory("spanish", "A1", root)).toThrow(/duplicate point id/);
    });
  });

  it("requires an exact, sourced boundary for every content dimension", () => {
    withTempInventory((root) => {
      const { lexicon: _lexicon, ...missingLexicon } = COMPLETE_SCOPE;
      write(root, { ...FIXTURE, scope: missingLexicon });
      expect(() => loadExamInventory("spanish", "A1", root)).toThrow(
        /scope must contain exactly.*missing \[lexicon\]/,
      );

      write(root, {
        ...FIXTURE,
        scope: {
          ...COMPLETE_SCOPE,
          grammar: { ...COMPLETE_SCOPE.grammar, source: "" },
        },
      });
      expect(() => loadExamInventory("spanish", "A1", root)).toThrow(
        /scope\.grammar\.source must name its provenance/,
      );
    });
  });

  it("loads and lists a partial inventory without claiming it is complete", () => {
    withTempInventory((root) => {
      write(root, {
        ...FIXTURE,
        scope: {
          ...COMPLETE_SCOPE,
          lexicon: { ...COMPLETE_SCOPE.lexicon, status: "partial" },
        },
      });
      expect(isExamInventoryComplete(loadExamInventory("spanish", "A1", root))).toBe(false);
      expect(listExamInventories(root)).toEqual([{ language: "spanish", level: "A1", complete: false }]);
    });
  });
});

describe("exam-inventory test ownership", () => {
  it("keeps language-specific committed-inventory suites out of this generic file", () => {
    const genericSource = readFileSync(new URL(import.meta.url), "utf8");
    expect(genericSource).not.toMatch(/describe\("the committed /);
  });
});
