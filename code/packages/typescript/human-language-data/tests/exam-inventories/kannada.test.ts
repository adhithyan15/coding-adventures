import { describe, expect, it } from "vitest";
import { loadTrackLessons, loadExamInventory } from "../../src/loader.js";
import {
  EXAM_CONTENT_DIMENSIONS,
  measureExamCoverage,
  formatExamCoverage,
  isExamInventoryComplete,
  trackIntroducedAtoms,
} from "../../src/exam-inventory.js";

describe("the committed Kannada A1 inventory", () => {
  const inventory = loadExamInventory("kannada", "A1");
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
    // This file was written while Kannada chapters 1, 2 and 4 were still
    // hand-written, so 37 of the atoms it probes did not exist when its first
    // draft was validated; the atom set was re-derived from the merged tree
    // before commit rather than trusted from the working branch.
    const lessons = loadTrackLessons("kannada");
    const taught = trackIntroducedAtoms(lessons, "kannada");
    const unknown: string[] = [];
    for (const point of inventory.points) {
      for (const atom of point.probe ?? []) if (!taught.has(atom)) unknown.push(`${point.id}:${atom}`);
    }
    expect(unknown).toEqual([]);
  }, 60_000);

  it("keeps the derivation total in both directions", () => {
    // Every Spanish point either derives into some Kannada point or is dropped
    // with a reason, and no point may be both. Fifteen were both in the first
    // draft, because restating a point around Kannada machinery FEELS like not
    // transferring it. Restating is deriving; `notTransferred` is now only the
    // two points that produce no Kannada point at all.
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
      const cast = point as unknown as { derivedFrom: string[]; kannadaSpecific?: boolean };
      expect(cast.kannadaSpecific === true, point.id).toBe(cast.derivedFrom.length === 0);
    }
    // Deliberately few, and that is a claim rather than an omission: nearly
    // every Kannada column answers a demand some Spanish point also makes, even
    // where the machinery is completely different -- a case suffix doing what a
    // preposition does, a dative subject doing what gustar does. Only four
    // points have no Spanish question behind them at all.
    const specific = inventory.points.filter(
      (point) => (point as unknown as { derivedFrom: string[] }).derivedFrom.length === 0,
    );
    expect(specific.map((point) => point.id).sort()).toEqual(
      ["KA-A1-CASE-02", "KA-A1-N-06", "KA-A1-P-05", "KA-A1-REG-04"],
    );
  });

  it("refuses to borrow an authority it does not have", () => {
    expect(inventory.about).toMatch(/PROJECT-DEFINED EDITORIAL EQUIVALENT, NOT AN EXTERNAL SYLLABUS/);
    expect(inventory.about).toMatch(/Kannada Sahitya Parishat/);
    expect(inventory.about).toMatch(/Karnataka's state school syllabi/);
    expect(inventory.source).toMatch(/^PROJECT-DEFINED\./);
    // Kannada has neither mocks nor task shapes nor an assessment contract, so
    // the file must say what it used instead. An unstated substitution is an
    // unauditable one.
    expect(inventory.about).toMatch(/EXAM ENVELOPE: NONE EXISTS/);
    expect(inventory.about).toMatch(/no kannada\/task-shapes\/ and no kannada\/mocks\//);
    expect(inventory.about).toMatch(/NOT SEARCHED, BY INSTRUCTION/);
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

  it("reports a CLOSED joining column and a script still 19 characters short", () => {
    // Pinned so a future tranche has to say which points it moved. It may rise;
    // a fall means coverage was lost and wants explaining.
    const lessons = loadTrackLessons("kannada");
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage.enumerated).toBe(258);
    // 194 -> 197: KA-A1-L-12 (the full stop and the comma), KA-A1-L-13 (the
    // question mark) and KA-A1-L-14 (colon, brackets, quotes, dash). THE CORPUS
    // HAD BEEN PRINTING THESE MARKS SINCE CHAPTER ONE while no lesson named any
    // of them -- every Kannada sentence in every reading passage ends in a Latin
    // full stop -- so chapter 77 opens by pointing at the end of a line in the
    // previous chapter and saying that something is sitting there nothing has
    // named. Kannada borrows the whole Latin set, shape and job together, which
    // is why three points cost one short chapter and nothing in it looks
    // unfamiliar.
    // L-13 CARRIES THE ONE LOAD-BEARING CONTRAST: the Spanish demand it derives
    // from opens a question with a second inverted mark and Kannada does not, so
    // a Kannada reader meets the mark at the end or not at all and the WORDS have
    // to carry the question until then.
    // L-14 IS PROBED AS A RECOGNITION POINT, not a production one: at A1 the
    // demand is knowing what a colon or a bracket signals on a notice, and the
    // recall lesson sorts the set into the three a reader writes and the rest
    // they read. KA-A1-L-15 (abbreviations and symbols) stays open.
    // FOUR OF KANNADA'S UNMAPPED POINTS ARE STRUCTURALLY UNCOVERABLE and are
    // marked untransferable in the inventory: capital letters, written
    // accentuation and superscript abbreviation letters have no Kannada
    // counterpart at all, and neither does Spanish's mid-distance demonstrative.
    // The real ceiling for this track is 254/258, not 258/258.
    expect(coverage.covered).toBe(198);
    expect(coverage.unmapped).toBe(60);
    expect(coverage.partial).toBe(0);
    // 193 -> 194: the HL-C354 ordinal tranche closed KA-A1-NUM-07 (chapters
    // 74-75). It is the ONLY point that moved, and the numeral column below
    // says so on its own line rather than leaving the total to speak for it.
    expect(coverage.byCategory["Sankhye (numerals and quantity)"]!).toEqual({
      enumerated: 8,
      covered: 7,
    });
    // WHAT THIS ASSERTION USED TO SAY, and why it changed. The inventory landed
    // reporting 167/258 and an EMPTY joining column: `mattu`, `athava`,
    // `aadare`, `eekendare` and the quotative `anta`/`endu` occurred ZERO times
    // in 268 lessons and zero times in the generated book, and the only two
    // covered points were the -i participle and the turn-level connectives of
    // chapter 64 -- a chapter literally named JOIN that joins turns and not
    // clauses. Chapters 67 to 73 answer that finding directly: 27 headwords
    // chosen off this file's OWN uncovered list closed 26 points, of which nine
    // are this column. The denominator did not move and `partial` stayed at 0,
    // so nothing was reworded to make the number rise.
    const joining = coverage.byCategory["Samuccaya (joining and subordination)"]!;
    expect(joining).toEqual({ enumerated: 11, covered: 11 });
    // DO NOT CARRY THE TAMIL SHAPE HERE. Tamil's script column came back 52 of
    // 52 characters taught. Kannada's is the opposite case and was measured
    // twice: 42 characters taught against 69 used in headwords when this
    // inventory was written, and 50 of 69 after chapters 67-73 taught the eight
    // most-used untaught characters. `ma` alone appears in 36 headwords and was
    // never taught. Nineteen characters remain, six of which have a sourced
    // ductus this project has not spent and thirteen of which have none.
    // THE COUNT ONCE MOVED 8 -> 11 WITHOUT THE CHARACTER DEBT SHRINKING BY ONE.
    // Those three points were PUNCTUATION -- the full stop and comma, the
    // question mark, and the colon/brackets/quotes/dash -- which Kannada borrows
    // whole from the Latin alphabet and which this column happens to house
    // alongside the characters. A count over the column is the wrong proxy for
    // "the script is still short", so the assertion named the thing it meant:
    // KA-A1-L-09, whose own label used to read "THE SCRIPT IS NOT CLOSED: N
    // characters are used but never taught", with N going 27, 19, 13.
    // IT IS NOW CLOSED, AND THE ASSERTION IS REWRITTEN RATHER THAN FLIPPED. A
    // bare `.covered` toBe(true) would pass on a probe that had been quietly
    // emptied, so the probe's own size is asserted beside it: the point carries
    // all TWENTY-SEVEN characters it was opened for -- the eight chapters 67-73
    // taught, the six chapter 78 taught as writing from cited stroke-order
    // animations, and the thirteen chapters 79 and 80 taught as recognition
    // because no ductus for them exists anywhere in this project.
    // The character count itself is pinned at an exact zero in
    // tests/corpus/kannada.test.ts, which is where a regression would show.
    const scriptClosed = coverage.points.find((p) => p.id === "KA-A1-L-09")!;
    expect(scriptClosed.covered).toBe(true);
    expect(
      loadExamInventory("kannada", "A1").points.find((p) => p.id === "KA-A1-L-09")!.probe,
    ).toHaveLength(27);
    expect(coverage.byCategory["Lipi (script and orthography)"]!.covered).toBe(12);
    // The two columns that carry this track, and they are not the ones French
    // and German lead on.
    expect(coverage.byCategory["Kriyaapada (the verb)"]!.covered).toBeGreaterThan(12);
    expect(coverage.byCategory["Padakosha (lexicon by domain)"]!.covered).toBeGreaterThan(45);
    expect(formatExamCoverage(coverage)).toContain(
      "kannada A1 (partial inventory): 198/258 points covered (77%)",
    );
  }, 60_000);
});
