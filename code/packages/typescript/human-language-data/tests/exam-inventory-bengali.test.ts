// ---------------------------------------------------------------------------
// The Bengali A1 inventory, in its own file.
//
// WHY NOT IN `exam-inventory.test.ts` WITH THE OTHER TRACKS
// Three inventories — Bengali, Arabic and Urdu — were written from three
// branches open at the same time. Each would have appended its describe block
// to the END of that file, and git conflicts on adjacent end-of-file additions
// however they are ordered, so the three would have collided pairwise for no
// reason other than where they were parked. The package already keeps per-track
// suites in their own files (`urdu-assessment`, `persian-task-shapes`,
// `tamil-inventory-ownership`); this follows that.
//
// Nothing here is weaker for living apart: it loads the committed file through
// the same strict `loadExamInventory` door and measures it against the same
// corpus as every block in the shared file.
// ---------------------------------------------------------------------------
import { describe, expect, it } from "vitest";
import { loadEverything, loadExamInventory } from "../src/loader.js";
import {
  EXAM_CONTENT_DIMENSIONS,
  measureExamCoverage,
  formatExamCoverage,
  isExamInventoryComplete,
  trackIntroducedAtoms,
} from "../src/exam-inventory.js";

describe("the committed Bengali A1 inventory", () => {
  const inventory = loadExamInventory("bengali", "A1");
  const spanish = loadExamInventory("spanish", "A1");

  it("keeps every point's probe key, and never an empty probe", () => {
    for (const point of inventory.points) {
      expect(point, `${point.id} has no probe key`).toHaveProperty("probe");
      expect(Array.isArray(point.probe) ? point.probe.length : 1, point.id).toBeGreaterThan(0);
    }
  });

  it("probes only atoms that EXIST, so a guessed id cannot under-report", () => {
    const { lessons } = loadEverything();
    const taught = trackIntroducedAtoms(lessons, "bengali");
    const unknown: string[] = [];
    for (const point of inventory.points) {
      for (const atom of point.probe ?? []) if (!taught.has(atom)) unknown.push(`${point.id}:${atom}`);
    }
    expect(unknown).toEqual([]);
  }, 60_000);

  it("keeps the derivation total in both directions", () => {
    const proxy = (inventory as unknown as {
      proxy: { notTransferred: { spanishPoints: string[]; why: string }[] };
    }).proxy;
    const dropped = new Set(proxy.notTransferred.flatMap((entry) => entry.spanishPoints));
    for (const entry of proxy.notTransferred) expect(entry.why.trim().length).toBeGreaterThan(0);
    const derived = new Set(
      inventory.points.flatMap((point) => (point as unknown as { derivedFrom: string[] }).derivedFrom),
    );
    const sourceIds = new Set(spanish.points.map((point) => point.id));
    for (const id of derived) expect(sourceIds.has(id), `derivedFrom names unknown ${id}`).toBe(true);
    for (const id of dropped) expect(sourceIds.has(id), `notTransferred names unknown ${id}`).toBe(true);
    expect([...derived].filter((id) => dropped.has(id)), "derived AND dropped").toEqual([]);
    const unaccounted = [...sourceIds].filter((id) => !derived.has(id) && !dropped.has(id));
    expect(unaccounted, "Spanish points that went missing from the walk").toEqual([]);
    // Only TWO Spanish points yield nothing at all. Restating a point around
    // Bengali's own machinery — a classifier where Spanish has an article, a
    // case suffix where Spanish has a preposition — is deriving it, which is the
    // lesson Kannada's first draft had to be corrected for.
    expect(dropped.size).toBe(2);
  });

  it("marks its own points as its own, in both directions", () => {
    for (const point of inventory.points) {
      const cast = point as unknown as { derivedFrom: string[]; bengaliSpecific?: boolean };
      expect(cast.bengaliSpecific === true, point.id).toBe(cast.derivedFrom.length === 0);
    }
    const specific = inventory.points.filter(
      (point) => (point as unknown as { derivedFrom: string[] }).derivedFrom.length === 0,
    );
    expect(specific.map((point) => point.id)).toEqual([
      "BN-A1-N-06", "BN-A1-V-13", "BN-A1-V-14", "BN-A1-KAR-05",
      "BN-A1-REG-02", "BN-A1-REG-03", "BN-A1-REG-04", "BN-A1-REG-05",
    ]);
  });

  it("says NO exam envelope exists, unlike Arabic's", () => {
    // Each `about` has to state its own envelope. Arabic's A1 task shape is
    // sourced from a real external test; Urdu's assessment.json points at an
    // a1.json that is not on disk; Bengali has no task-shapes/, no mocks/ and no
    // assessment.json at all. Copying a sibling's sentence here would claim an
    // envelope this track has not got.
    expect(inventory.about).toMatch(/PROJECT-DEFINED EDITORIAL EQUIVALENT, NOT AN EXTERNAL SYLLABUS/);
    expect(inventory.about).toMatch(/EXAM ENVELOPE: NONE EXISTS/);
    expect(inventory.about).toMatch(/NOT SEARCHED, BY INSTRUCTION/);
    expect(inventory.source).toMatch(/^PROJECT-DEFINED\./);
    expect(isExamInventoryComplete(inventory)).toBe(false);
    for (const dimension of EXAM_CONTENT_DIMENSIONS) {
      expect(inventory.scope[dimension].status, dimension).toBe("partial");
    }
  });

  it("derives its register column from lesson BODIES, because BOTH frontmatter fields are artefacts", () => {
    // All 139 lessons declare `register: neutral`, and `variety` takes three
    // values that carry no register information at all: "standard" for every
    // writing lesson, "standard-bengali" for chapters C01-C05, and
    // "standard-colloquial" for C06-C15 — predicted exactly by the lesson id
    // prefix, which is to say by when the lesson was written. Tamil's register
    // column was NOT imported here; this one was measured.
    expect(inventory.about).toMatch(
      /THE REGISTER COLUMN IS DERIVED FROM LESSON BODIES, NOT FROM FRONTMATTER, AND THE VARIETY FIELD IS AN ARTEFACT/,
    );
    const register = inventory.points.filter((point) => point.category.startsWith("Bhashar star"));
    expect(register).toHaveLength(5);
    // exam-levels.json carries NO caveat for bengali. The absence is recorded as
    // a point rather than filled in from Tamil's.
    expect(register.some((point) => point.id === "BN-A1-REG-05")).toBe(true);
  });

  it("names an anchor for every point, and says what kind of anchor it is", () => {
    const anchors = (inventory as unknown as {
      anchors: { id: string; kind: string; title: string; note: string }[];
    }).anchors;
    expect(new Set(anchors.map((anchor) => anchor.kind))).toEqual(
      new Set(["sourced-proxy", "external-framework", "project-owned", "editorial"]),
    );
    for (const anchor of anchors) expect(anchor.note.trim().length, anchor.id).toBeGreaterThan(0);
    const known = new Set(anchors.map((anchor) => anchor.id));
    for (const point of inventory.points) {
      const ids = (point as unknown as { anchorIds?: string[] }).anchorIds;
      expect(ids?.length, `${point.id} names no anchor`).toBeGreaterThan(0);
      for (const id of ids ?? []) expect(known.has(id), `${point.id} cites unknown anchor ${id}`).toBe(true);
    }
  });

  it("reports an EMPTY joining column, no demonstrative at all, and a script 11 glyphs short", () => {
    const { lessons } = loadEverything();
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage.enumerated).toBe(244);
    expect(coverage.covered).toBe(104);
    expect(coverage.unmapped).toBe(140);
    expect(coverage.partial).toBe(0);
    // The headline, and the fifth empty joining column measured in this series.
    // আর and এবং ("and"), কিন্তু ("but"), কারণ ("because"), যে ("that") and যখন
    // ("when") all return ZERO occurrences in 139 files, and every raw match for
    // বা ("or") was checked in context and is a substring of ভাবা or বোঝা. The
    // ONE covered point is covered by accident: দয়া করে teaches the conjunctive
    // participle because "please" happens to be built out of one.
    expect(coverage.byCategory["Shomuchchoy (joining and subordination)"]!).toEqual({
      enumerated: 11,
      covered: 1,
    });
    // Nothing can be pointed at: এই and ওই both return zero matches.
    expect(coverage.byCategory["Nirdeshak (demonstratives and deixis)"]!).toEqual({
      enumerated: 3,
      covered: 0,
    });
    // The script column is this track's strength AND carries its hardest gap.
    // measureScriptClosure reports 34 glyphs taught of 45 shown, 11 never
    // taught, 21 violations, and headwordsWithoutRomanization exactly 0 — and
    // separately, no Bengali ductus data exists anywhere in the repository, so
    // stroke order cannot be taught at all (HL-C212).
    expect(coverage.byCategory["Lipi (script and orthography)"]!).toEqual({
      enumerated: 16,
      covered: 8,
    });
    expect(formatExamCoverage(coverage)).toContain(
      "bengali A1 (partial inventory): 104/244 points covered (43%)",
    );
  }, 60_000);
});
