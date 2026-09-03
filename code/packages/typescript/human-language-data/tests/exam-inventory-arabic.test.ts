// ---------------------------------------------------------------------------
// The Arabic A1 inventory, in its own file.
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

describe("the committed Arabic A1 inventory", () => {
  const inventory = loadExamInventory("arabic", "A1");
  const spanish = loadExamInventory("spanish", "A1");

  it("keeps every point's probe key, and never an empty probe", () => {
    for (const point of inventory.points) {
      expect(point, `${point.id} has no probe key`).toHaveProperty("probe");
      expect(Array.isArray(point.probe) ? point.probe.length : 1, point.id).toBeGreaterThan(0);
    }
  });

  it("probes only atoms that EXIST, so a guessed id cannot under-report", () => {
    const { lessons } = loadEverything();
    const taught = trackIntroducedAtoms(lessons, "arabic");
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
    // Exactly ONE Spanish point yields nothing at all. Restating a point around
    // Arabic's own machinery — a broken plural where Spanish suffixes, a
    // verbless sentence where Spanish needs ser — is deriving it.
    expect([...dropped]).toEqual(["A1-PRON-07"]);
  });

  it("marks its own points as its own, in both directions", () => {
    for (const point of inventory.points) {
      const cast = point as unknown as { derivedFrom: string[]; arabicSpecific?: boolean };
      expect(cast.arabicSpecific === true, point.id).toBe(cast.derivedFrom.length === 0);
    }
    const specific = inventory.points.filter(
      (point) => (point as unknown as { derivedFrom: string[] }).derivedFrom.length === 0,
    );
    expect(specific.map((point) => point.id)).toEqual([
      "AR-A1-N-07", "AR-A1-N-08", "AR-A1-V-03", "AR-A1-V-06",
      "AR-A1-REG-01", "AR-A1-REG-02", "AR-A1-REG-03", "AR-A1-REG-04",
    ]);
  });

  it("names an EXTERNALLY SOURCED task shape that still prescribes no content", () => {
    // Each `about` has to state its own envelope. Bengali has no task-shapes/,
    // no mocks/ and no assessment.json at all; Urdu's assessment.json points at
    // an a1.json that is not on disk. Arabic is the only track in this series
    // whose A1 task shape carries `basis: external` — and STAMP is adaptive, so
    // it describes a delivery mechanism and names no item. Claiming it as a
    // content source would be claiming more than the file supports.
    expect(inventory.about).toMatch(/PROJECT-DEFINED EDITORIAL EQUIVALENT, NOT AN EXTERNAL SYLLABUS/);
    expect(inventory.about).toMatch(
      /EXAM ENVELOPE: AN EXTERNALLY-SOURCED A1 TASK SHAPE EXISTS AND PRESCRIBES NO CONTENT/,
    );
    expect(inventory.about).toMatch(/There is no arabic\/mocks\//);
    expect(inventory.about).toMatch(/NOT REPEATED, BY INSTRUCTION/);
    expect(inventory.source).toMatch(/^PROJECT-DEFINED\./);
    expect(isExamInventoryComplete(inventory)).toBe(false);
    for (const dimension of EXAM_CONTENT_DIMENSIONS) {
      expect(inventory.scope[dimension].status, dimension).toBe("partial");
    }
  });

  it("answers the diglossia caveat by MEASURING it, and finds it overstated", () => {
    // tracks.arabic is the only entry in this series whose caveat makes a claim
    // ABOUT THE CORPUS: "the corpus also teaches greetings a learner would hear
    // in dialect". All 102 lessons declare variety "modern-standard-arabic" —
    // the field has no second value — and the two dialect remarks that exist
    // introduce no atom. The file records that rather than agreeing with the
    // caveat it was handed.
    expect(inventory.about).toMatch(/THE DIGLOSSIA CAVEAT WAS MEASURED, NOT REPEATED/);
    const register = inventory.points.filter((point) => point.category.startsWith("Al-fusha"));
    expect(register).toHaveLength(5);
    const dialect = register.find((point) => point.id === "AR-A1-REG-02")!;
    expect(dialect.probe).toBeNull();
    expect(dialect.note).toMatch(/OVERSTATES WHAT IS HERE/);
    // Arabic splits its second person by GENDER, not by respect, so this track
    // has no tu/usted or tumi/apni ladder to measure. Tamil's register column
    // was not imported here for exactly that reason.
    expect(register.find((point) => point.id === "AR-A1-REG-05")!.note).toMatch(
      /split by GENDER rather than by respect/,
    );
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

  it("reports a joining column of ZERO, and the worst script closure in the series", () => {
    const { lessons } = loadEverything();
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage.enumerated).toBe(238);
    expect(coverage.covered).toBe(106);
    expect(coverage.unmapped).toBe(132);
    expect(coverage.partial).toBe(0);
    // THE HEADLINE, and the first column in this whole series to measure a flat
    // zero. و — the commonest word in written Arabic — is GLOSSED once, in
    // chapter 1's fixed reply ("hear the little wa- at the front; it means
    // and"), and that sentence introduces no atom. أو's single raw match is the
    // middle of الأول; لكن, لأن, أنّ and عندما all return zero occurrences.
    expect(coverage.byCategory["Ar-rabt (joining and subordination)"]!).toEqual({
      enumerated: 11,
      covered: 0,
    });
    // Nothing can be pointed at: هذا and هذه both return zero occurrences.
    expect(coverage.byCategory["Al-ishara (demonstratives and deixis)"]!).toEqual({
      enumerated: 3,
      covered: 0,
    });
    // measureScriptClosure: 30 glyphs taught of 45 shown, 15 never taught, 57
    // violations across 102 lessons, and headwordsWithoutRomanization 37 —
    // against 0 for both Bengali and Urdu. ف alone is shown in 34 lessons and
    // taught in none. All eighteen writing lessons sit in chapters 1-4.
    expect(coverage.byCategory["Al-khatt (script and orthography)"]!).toEqual({
      enumerated: 12,
      covered: 4,
    });
    // The other end. Arabic's time lexicon and its root-and-pattern engine are
    // the strongest columns measured anywhere in this series.
    expect(coverage.byCategory["Al-ism (the noun)"]!).toEqual({ enumerated: 8, covered: 7 });
    expect(formatExamCoverage(coverage)).toContain(
      "arabic A1 (partial inventory): 106/238 points covered (45%)",
    );
  }, 60_000);
});
