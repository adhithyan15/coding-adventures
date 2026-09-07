// ---------------------------------------------------------------------------
// The Italian A1 inventory, in its own file.
//
// WHY NOT IN `exam-inventory.test.ts` WITH THE OTHER TRACKS
// Five inventories — Italian, Latin, Marwadi, Persian and Portuguese — were the
// five tracks with no inventory at all, and they were written from five
// branches. Each would have appended its describe block to the END of that
// file, and git conflicts on adjacent end-of-file additions however they are
// ordered, so they would have collided pairwise for no reason other than where
// they were parked. Bengali, Arabic and Urdu already live apart for exactly
// this reason; this follows them.
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

describe("the committed Italian A1 inventory", () => {
  const inventory = loadExamInventory("italian", "A1");
  const spanish = loadExamInventory("spanish", "A1");

  it("keeps every point's probe key, and never an empty probe", () => {
    for (const point of inventory.points) {
      expect(point, `${point.id} has no probe key`).toHaveProperty("probe");
      expect(Array.isArray(point.probe) ? point.probe.length : 1, point.id).toBeGreaterThan(0);
    }
  });

  it("gives every unmapped point a note, because an unmapped point IS the work queue", () => {
    for (const point of inventory.points) {
      if (point.probe !== null) continue;
      expect(point.note?.trim().length, `${point.id} is unmapped and says nothing`).toBeGreaterThan(0);
    }
  });

  it("probes only atoms that EXIST, so a guessed id cannot under-report", () => {
    const { lessons } = loadEverything();
    const taught = trackIntroducedAtoms(lessons, "italian");
    const unknown: string[] = [];
    for (const point of inventory.points) {
      for (const atom of point.probe ?? []) if (!taught.has(atom)) unknown.push(`${point.id}:${atom}`);
    }
    expect(unknown).toEqual([]);
  }, 60_000);

  it("drops NOTHING, because the proxy and the target are sister languages", () => {
    const proxy = (inventory as unknown as {
      proxy: { notTransferred: { spanishPoints: string[]; why: string }[]; note: string };
    }).proxy;
    const dropped = new Set(proxy.notTransferred.flatMap((entry) => entry.spanishPoints));
    const derived = new Set(
      inventory.points.flatMap((point) => (point as unknown as { derivedFrom: string[] }).derivedFrom),
    );
    const sourceIds = new Set(spanish.points.map((point) => point.id));
    for (const id of derived) expect(sourceIds.has(id), `derivedFrom names unknown ${id}`).toBe(true);
    expect([...derived].filter((id) => dropped.has(id)), "derived AND dropped").toEqual([]);
    const unaccounted = [...sourceIds].filter((id) => !derived.has(id) && !dropped.has(id));
    expect(unaccounted, "Spanish points that went missing from the walk").toEqual([]);
    // The claim this file is willing to make that no sibling inventory can:
    // Italian and Spanish are close enough that every one of the 273 proxy
    // points names a real Italian demand. Restating a point around the target's
    // machinery — a final-stress accent where Spanish has an exception-marking
    // one, the ABSENCE of the inverted question mark — is deriving it, which is
    // the lesson Kannada's first draft had to be corrected for, carried to its
    // end. An empty list has to say so out loud or it reads as a walk not done.
    expect(dropped.size).toBe(0);
    expect(proxy.note).toMatch(/EMPTY ON PURPOSE/);
    expect(proxy.note).toMatch(/RESTATING A POINT AROUND\s+THE TARGET LANGUAGE'S MACHINERY IS DERIVING IT/);
  });

  it("marks its own points as its own, in both directions", () => {
    for (const point of inventory.points) {
      const cast = point as unknown as { derivedFrom: string[]; italianSpecific?: boolean };
      expect(cast.italianSpecific === true, point.id).toBe(cast.derivedFrom.length === 0);
    }
    const specific = inventory.points.filter(
      (point) => (point as unknown as { derivedFrom: string[] }).derivedFrom.length === 0,
    );
    expect(specific.map((point) => point.id)).toEqual([
      "IT-A1-N-07", "IT-A1-N-08", "IT-A1-PRON-03", "IT-A1-F2-17",
      "IT-A1-REG-01", "IT-A1-REG-02", "IT-A1-REG-03", "IT-A1-REG-04",
    ]);
  });

  it("says NO exam envelope exists here, while an external exam DOES exist", () => {
    // Italian is not Tamil. exam-levels.json records a real published exam
    // (CILS / CELI) with no caveat at all, so this file may not borrow Tamil's
    // "no ladder exists" sentence — and it may not claim the exam either,
    // because nobody here has read a CILS content syllabus. The envelope claim
    // is about THIS REPOSITORY: italian/ ships no assessment.json, no
    // task-shapes/ and no mocks/, unlike latin/ and persian/.
    expect(inventory.about).toMatch(/PROJECT-DEFINED EDITORIAL EQUIVALENT, NOT AN EXTERNAL SYLLABUS/);
    expect(inventory.about).toMatch(/NOT SEARCHED, BY INSTRUCTION/);
    expect(inventory.about).toMatch(/CILS/);
    expect(inventory.source).toMatch(/^PROJECT-DEFINED\./);
    expect(inventory.source).toMatch(/EXAM ENVELOPE: NONE EXISTS IN THIS REPOSITORY/);
    expect(isExamInventoryComplete(inventory)).toBe(false);
    for (const dimension of EXAM_CONTENT_DIMENSIONS) {
      expect(inventory.scope[dimension].status, dimension).toBe("partial");
    }
  });

  it("derives its register column from lesson BODIES, because exam-levels.json carries NO caveat", () => {
    // Both tracks in this batch with a published external exam — italian and
    // portuguese — carry no caveat, so there is nothing to import and nothing
    // to defer to. Unlike Bengali's, italian's `register` frontmatter is not a
    // pure artefact: six of 93 lessons declare something other than `neutral`.
    // It is still not an index, because it misses come-stai, come-ti-chiami and
    // buongiorno, all of which teach the tu/Lei contrast in their bodies.
    expect(inventory.about).toMatch(
      /THE REGISTER COLUMN IS DERIVED FROM LESSON BODIES, NOT FROM FRONTMATTER, AND THE FRONTMATTER FIELD IS\s+HALF-APPLIED RATHER THAN AN ARTEFACT/,
    );
    const register = inventory.points.filter((point) => point.category.startsWith("Registro"));
    expect(register).toHaveLength(4);
    expect(register.some((point) => point.id === "IT-A1-REG-04")).toBe(true);
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

  it("reports a joining column of ONE, a track that cannot say no, and no punctuation taught", () => {
    const { lessons } = loadEverything();
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage.enumerated).toBe(292);
    expect(coverage.covered).toBe(111);
    expect(coverage.unmapped).toBe(181);
    expect(coverage.partial).toBe(0);
    // The joining column, which has been near-empty in every track measured.
    // Italian's ONE covered point is covered sideways: chapter 16's lesson on
    // the written accent has to gloss plain `e` as "and" to contrast it with
    // `è`, and chapter 5's dialogue then uses it in "Abito a Roma, e lavoro a
    // Milano". `ma` returns one match in 93 files and it is inside the Latin
    // `de mane`; `perché`, `quando`, `né` return zero; every raw match for `o`
    // is a letter named in a pronunciation note.
    expect(coverage.byCategory["Congiunzione e subordinazione (joining and subordination)"]!).toEqual({
      enumerated: 13,
      covered: 1,
    });
    // Nothing can be pointed at and nothing can be owned.
    expect(coverage.byCategory["I dimostrativi (pointing)"]!).toEqual({ enumerated: 3, covered: 0 });
    expect(coverage.byCategory["I possessivi (possession)"]!).toEqual({ enumerated: 3, covered: 0 });
    // The verb column is this track's strength — three regular classes, two
    // pasts, four irregular verbs — and the noun column is closed outright.
    expect(coverage.byCategory["Il verbo (the verb)"]!).toEqual({ enumerated: 19, covered: 15 });
    expect(coverage.byCategory["Il sostantivo (the noun)"]!).toEqual({ enumerated: 8, covered: 8 });
    expect(formatExamCoverage(coverage)).toContain(
      "italian A1 (partial inventory): 111/292 points covered (38%)",
    );
  }, 60_000);

  it("keeps the two findings that no coverage percentage would surface", () => {
    // (1) NEGATION. `non` occurs in 2 of 93 files, both inside the frozen
    //     `non c'è male`, whose own lesson defers negation by name. Everything
    //     that needs a negator is therefore unreachable, and that is a longer
    //     list than the negation point itself.
    const negation = inventory.points.find((point) => point.id === "IT-A1-OS-02")!;
    expect(negation.probe).toBeNull();
    expect(negation.note).toMatch(/MOST CONSEQUENTIAL GAP/);
    // (2) REPAIR. Checked separately from coverage, and Italian HAS one:
    //     rising `Prego?` gets a lesson of its own as a listening repair.
    const repair = inventory.points.find((point) => point.id === "IT-A1-F6-05")!;
    expect(repair.probe).toEqual(["IT-PRAGMATIC-PREGO-COME-AGAIN-06"]);
    // …and the other half of the repair column is not there: a learner can say
    // `capisco` and cannot say `non capisco`, because of (1).
    const understanding = inventory.points.find((point) => point.id === "IT-A1-F2-17")!;
    expect(understanding.probe).toBeNull();
  });
});
