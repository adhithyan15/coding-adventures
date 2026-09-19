import { describe, expect, it } from "vitest";
import { loadTrackLessons, loadExamInventory } from "../../src/loader.js";
import {
  EXAM_CONTENT_DIMENSIONS,
  measureExamCoverage,
  formatExamCoverage,
  isExamInventoryComplete,
  trackIntroducedAtoms,
} from "../../src/exam-inventory.js";

describe("the committed Tamil A1 inventory", () => {
  const inventory = loadExamInventory("tamil", "A1");
  const spanish = loadExamInventory("spanish", "A1");

  it("keeps every point's probe key, and never an empty probe", () => {
    for (const point of inventory.points) {
      expect(point, `${point.id} has no probe key`).toHaveProperty("probe");
      expect(Array.isArray(point.probe) ? point.probe.length : 1, point.id).toBeGreaterThan(0);
    }
  });

  it("probes only atoms that EXIST, so a guessed id cannot under-report", () => {
    // The rule HL-C290 calls out by name. A probe pointing at an id somebody
    // expects a future lesson to introduce resolves to "not introduced" forever
    // and sits in the report indistinguishable from the honest gaps around it.
    const lessons = loadTrackLessons("tamil");
    const taught = trackIntroducedAtoms(lessons, "tamil");
    const unknown: string[] = [];
    for (const point of inventory.points) {
      for (const atom of point.probe ?? []) if (!taught.has(atom)) unknown.push(`${point.id}:${atom}`);
    }
    expect(unknown).toEqual([]);
  }, 60_000);

  it("keeps the derivation total in both directions", () => {
    // Every Spanish point either derives into some Tamil point or is dropped
    // with a reason, and no point may be both. A source point that is silently
    // absent is indistinguishable from one nobody thought of, which is the
    // failure the whole exercise exists to prevent.
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
  });

  it("marks its own points as its own, in both directions", () => {
    for (const point of inventory.points) {
      const cast = point as unknown as { derivedFrom: string[]; tamilSpecific?: boolean };
      expect(cast.tamilSpecific === true, point.id).toBe(cast.derivedFrom.length === 0);
    }
    const specific = inventory.points.filter(
      (point) => (point as unknown as { derivedFrom: string[] }).derivedFrom.length === 0,
    );
    // A proxy is a scaffold, not a template. Tamil's own points include the
    // rational/irrational split that governs all its agreement, the stacking
    // case suffix, the strong/weak verb sort, the two-way negative, the three
    // n letters, and the whole register column.
    expect(specific.length).toBeGreaterThanOrEqual(20);
  });

  it("refuses to borrow an authority it does not have", () => {
    // Three bodies could lend this file weight it has not earned: the two the
    // proficiency backbone names as the nearest thing to a Tamil ladder, and
    // the awarding body behind the proxy.
    expect(inventory.about).toMatch(/PROJECT-DEFINED EDITORIAL EQUIVALENT, NOT AN EXTERNAL SYLLABUS/);
    expect(inventory.about).toMatch(/Singapore Ministry of Education/);
    expect(inventory.about).toMatch(/Tamil Nadu state syllabi/);
    expect(inventory.about).toMatch(/MAY BE ATTRIBUTED TO DELE/);
    // The search is settled per HL-C287/HL-C290 and was deliberately not redone.
    // Saying so is what keeps "we did not look" from reading as "there is
    // nothing to find".
    expect(inventory.about).toMatch(/NOT SEARCHED, BY INSTRUCTION/);
    expect(inventory.source).toMatch(/^PROJECT-DEFINED\./);
    // Tamil has neither mocks nor task shapes, so the file must say what it
    // used instead of the artifacts Hindi and Marathi mined. An unstated
    // substitution is an unauditable one.
    expect(inventory.source).toMatch(/EXAM ENVELOPE: NONE EXISTS/);
    expect(inventory.about).toMatch(/no tamil\/task-shapes\/ and no tamil\/mocks\//);
    expect(isExamInventoryComplete(inventory)).toBe(false);
    for (const dimension of EXAM_CONTENT_DIMENSIONS) {
      expect(inventory.scope[dimension].status, dimension).toBe("partial");
    }
  });

  it("names an anchor for every point, and says what kind of anchor it is", () => {
    const anchors = (inventory as unknown as {
      anchors: { id: string; kind: string; title: string; note: string }[];
    }).anchors;
    expect(Array.isArray(anchors)).toBe(true);
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

  it("reports a gap that is GRAMMAR-shaped, with the script column nearly closed", () => {
    // Pinned so a future tranche has to say which points it moved. It may rise;
    // a fall means coverage was lost and wants explaining.
    //
    // 155 -> 174 (HL-C304, chapters 74-81). The clause-joining tranche closed
    // the whole Iṇaittoḍar column plus the polar -ā, eṉ, eppōdu, the additive
    // -um, negative coordination, four communicative functions, both numeral
    // points — by declaring the atoms chapter 7 was already teaching — and the
    // diglossia point the file named as the most Tamil-specific one in it.
    const lessons = loadTrackLessons("tamil");
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage.enumerated).toBe(262);
    // 175 -> 176: TA-A1-PRON-03, the rest of the subject paradigm. THE GAP WAS
    // INSIDE A RULE THAT HAD ALREADY BEEN TAUGHT: TA-C37-ivar teaches the i-/a-
    // pointing system and its table says in as many words that a- points away --
    // and the a- column had never been filled, so a reader could state what a-
    // meant and had no a- person word to say. avar now sits one lesson after
    // ivar, where that table shows the column; avan and aval are the familiar
    // pair ivar's own lesson already named as what it was built from; avarkaL is
    // avar plus the plural -kaL the reader has been pronouncing inside niingaL
    // since chapter two. naam against naangaL is the point the note called out as
    // the one Spanish does not make, and it is taught as a QUESTION -- is my
    // listener inside this 'we' -- rather than as a pair of words.
    // TA-A1-V-06'S NOTE WAS HALF WRONG AND IS CORRECTED IN THE INVENTORY: it said
    // 'no lesson puts a verb into the past', and TA-C32-po prints poogiReen /
    // pooneen / pooveen in a three-row table and glosses pooneen as 'I went'. The
    // real gap is PRODUCTIVITY, not exposure -- shown for one verb, never taught
    // as an atom -- which is a different and cheaper problem than described.
    // 176 -> 177: TA-A1-PRON-04, the accusative -ai on a person -- Tamil's answer
    // to Spanish's personal a, and the track's SECOND case ending after the
    // dative -ukku. The point's note was accurate and was verified before the
    // chapter was written: -ukku really was the only case taught anywhere.
    // THE GAP IS STILL GRAMMAR-SHAPED, which is what this test is named for, and
    // the pronoun column below moves on its own line rather than leaving the
    // total to speak for it.
    expect(coverage.covered).toBe(177);
    expect(coverage.unmapped).toBe(85);
    expect(coverage.byCategory["Pratippeyar (pronouns)"]!).toEqual({
      enumerated: 9,
      covered: 6,
    });
    // 174 -> 175: the HL-C354 ordinal tranche closed TA-A1-NUM-04 (chapters
    // 82-83). It is the ONLY point that moved, and the numeral column below
    // says so on its own line rather than leaving the total to speak for it.
    // The two still open in that column are TA-A1-NUM-03 (above twenty) and
    // TA-A1-NUM-07 (measures); neither is ordinal work.
    expect(coverage.byCategory["Eṇṇuppeyar (numerals and quantity)"]!).toEqual({
      enumerated: 8,
      covered: 6,
    });
    // Zero partials is a property of the "existing atoms only" rule, not a
    // coincidence: with no guessed ids, a point is either fully probed or null.
    expect(coverage.partial).toBe(0);
    // WAS THE FINDING, AND IS NOW THE PAYMENT. Tamil could not join two clauses
    // at all — no `-um ... -um`, no `aanaal`, no `alladu`, no quotative `enru`.
    // Chapters 74-81 teach all seven, which is what lets the well-taught verb
    // and lexis columns become sentences.
    expect(coverage.byCategory["Iṇaittoḍar (joining clauses)"]).toEqual({ enumerated: 7, covered: 7 });
    // The two columns that carry this track, and they are not the ones French
    // and German lead on.
    expect(coverage.byCategory["Vinaiccol (the verb)"]!.covered).toBeGreaterThan(15);
    expect(coverage.byCategory["Tamiḻ eḻuttu (script and orthography)"]!.covered).toBeGreaterThan(5);
    expect(formatExamCoverage(coverage)).toContain(
      "tamil A1 (partial inventory): 177/262 points covered (68%)",
    );
  }, 60_000);
});
