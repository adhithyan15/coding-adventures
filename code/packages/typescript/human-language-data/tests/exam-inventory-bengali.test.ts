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
    // All 139 lessons at the time of measurement declared `register: neutral`, and `variety` takes three
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

  it("reports THREE full columns, a closed deixis grid, and a script two glyphs shorter", () => {
    const { lessons } = loadEverything();
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage.enumerated).toBe(244);
    // 104 -> 121: the negation-and-joining tranche (chapters 27-30). Fifteen
    // items closed seventeen points, and the ratio comes from the two columns
    // being downstream of each other rather than from vocabulary -- the same
    // না that negates a verb is the না inside কেননা and the না doubled in
    // না … না, so three points ride on one word the track taught in chapter one
    // and never used again.
    //
    // 121 -> 138: the pointing tranche (chapters 31-37). Twenty-eight items
    // closed seventeen points, and this ratio comes from a LETTER. ট was shown
    // in the corpus and taught nowhere, and six points sat behind it: the
    // classifier, the indefinite, the distributive pair, the demonstrative
    // stacking rule and এটা. Teaching one consonant unblocked a whole grammar.
    //
    // 138 -> 139: the ordinal tranche (chapter 38). Nine items closed ONE
    // point, BN-A1-Q-04, and the ratio is the opposite of the one above: every
    // one of the five ordinals is a separate borrowed word, none of them
    // unblocks anything else, and the sixth item is the letter ঞ that পঞ্চম
    // needed. An ordinal column is expensive in a language that borrowed the
    // whole of it.
    //
    // 139 -> 141: the digit tranche (chapter 39). Twelve items closed TWO
    // points, and the two are the same piece of work filed in two columns --
    // BN-A1-Q-03 in the numeral column and BN-A1-LIP-11 in the script column,
    // exactly as the LIP-11 note said would happen. Neither total shows the
    // thing that actually changed: before this chapter the track taught 60
    // script lessons and not one of them put a NON-LETTER on the page, so no
    // price, date, clock face or page number was readable at all.
    expect(coverage.covered).toBe(141);
    expect(coverage.unmapped).toBe(103);
    expect(coverage.partial).toBe(0);
    // The headline was an EMPTY joining column: আর and এবং ("and"), কিন্তু
    // ("but"), কারণ ("because"), যে ("that") and যখন ("when") all returned ZERO
    // occurrences in 139 files, and the ONE covered point was covered by
    // accident -- দয়া করে teaches the conjunctive participle because "please"
    // happens to be built out of one. 8 -> 9: JOIN-05 named the classifier as
    // its blocker and the classifier now exists, so একটা … আরেকটা stands on a
    // lesson. The two that remain still name blockers rather than omissions:
    // the purpose suffix for JOIN-10, and the যে … সে correlative for JOIN-07.
    expect(coverage.byCategory["Shomuchchoy (joining and subordination)"]!).toEqual({
      enumerated: 11,
      covered: 9,
    });
    // Negation goes from 1/5 to 5/5 -- the first column in this track to close
    // completely. "I do not understand" is now sayable, and it is sayable in
    // romanization only, because ঝ still has no letter lesson.
    expect(coverage.byCategory["Nishedh (negation)"]!).toEqual({
      enumerated: 5,
      covered: 5,
    });
    // THE HEADLINE, and a flat zero closing to a full column. এই and ওই both
    // returned zero matches across 139 files; both are now taught, they stack
    // with the classifier at the opposite end of the phrase, and এটা stands
    // alone as a subject so that এটা কী? -- the commonest beginner sentence in
    // any language -- is sayable for the first time in thirty-two chapters.
    expect(coverage.byCategory["Nirdeshak (demonstratives and deixis)"]!).toEqual({
      enumerated: 3,
      covered: 3,
    });
    // Two more columns close completely alongside it, and neither was aimed at.
    // Definiteness closes because the classifier IS the definite article
    // Bengali has not got; possession closes because a third-person pronoun and
    // a plural one both take the same -র the track already owned.
    expect(coverage.byCategory["Nirdeshok (definiteness, and the article Bengali has not got)"]!).toEqual({
      enumerated: 3,
      covered: 3,
    });
    expect(coverage.byCategory["Odhikar (possession)"]!).toEqual({
      enumerated: 3,
      covered: 3,
    });
    // The pronoun column, one point off full: only the reflexive নিজে is left.
    expect(coverage.byCategory["Shorbonam (pronouns)"]!).toEqual({
      enumerated: 8,
      covered: 7,
    });
    // The script column is this track's strength AND carries its hardest gap.
    // measureScriptClosure now reports 36 glyphs taught of 45 shown, NINE never
    // taught, 21 violations, and headwordsWithoutRomanization exactly 0. The
    // violation count and the romanization count are unchanged by the
    // thirty-five lessons of chapters 31-37; the never-taught count FELL by two,
    // because this tranche needed ট and থ and could not write itself without
    // them. Both are consonants, and Commons holds a stroke-order animation for
    // every Bengali independent vowel and for no consonant at all (HL-C212), so
    // both are taught the way the corpus's other consonant lessons are: place
    // of articulation, the square they complete, and a Unicode chart citation.
    // No pen path is claimed, because none can be sourced.
    // 8 -> 9 with chapter 39's digits: the ONE point in this column that was
    // never about a letter. measureScriptClosure now reports 47 glyphs taught,
    // ten more than before and every one of them a digit, with violations and
    // never-taught BOTH unchanged at 21 and 8 -- the tranche neither fixed nor
    // added a single letter debt. The digits have no citable stroke order
    // either: HL-C212 found Commons carries animations for every Bengali VOWEL
    // and none for any consonant, and the same search finds nothing for the
    // digits, so each lesson teaches the shape against something the reader
    // already holds and claims NO PEN PATH.
    expect(coverage.byCategory["Lipi (script and orthography)"]!).toEqual({
      enumerated: 16,
      covered: 9,
    });
    expect(formatExamCoverage(coverage)).toContain(
      "bengali A1 (partial inventory): 141/244 points covered (58%)",
    );
  }, 60_000);

  // The ordinal point, named rather than left to the aggregate. Both halves were
  // falsified before this was kept: a fabricated id fails the "probes only atoms
  // that EXIST" test above, and nulling the probe fails the coverage total.
  it("closes BN-A1-Q-04 on nine atoms, five borrowed words and a letter", () => {
    const { lessons } = loadEverything();
    const taught = trackIntroducedAtoms(lessons, "bengali");
    const ordinals = inventory.points.find((point) => point.id === "BN-A1-Q-04");
    expect(ordinals?.probe).toEqual([
      // The five words, in numerical order here and in NO other order in the
      // chapter: it teaches second, third, fourth, first, fifth.
      "BN-LEX-C27-PROTHOM-01",
      "BN-LEX-C27-DITIYO-01",
      "BN-LEX-C27-TRITIYO-01",
      "BN-LEX-C27-CHOTURTHO-01",
      "BN-LEX-C27-PONCHOM-01",
      // That all five are tatsama -- borrowed back out of Sanskrit -- which is
      // the fact the whole chapter hangs on and the reason it opens on SECOND:
      // chapter 12 had already told the reader that the old dv- survives only
      // in re-borrowed words.
      "BN-GRAMMAR-C27-ORDINAL-TATSAMA-01",
      // The -তীয় shape, claimed on তৃতীয় rather than on দ্বিতীয় because one
      // word is not a shape.
      "BN-GRAMMAR-C27-ORDINAL-TIYO-01",
      // The payoff: পাঁচ is পঞ্চ with the nasal worn down into the chandrabindu.
      "BN-ETYMON-C27-PONCHO-PANCH-01",
    ]);
    for (const atom of ordinals?.probe ?? []) expect(taught.has(atom), atom).toBe(true);
    // পঞ্চম could not be written before this tranche: ঞ was shown in the corpus
    // and taught nowhere. It is taught now, and that is what carries the word.
    expect(taught.has("BN-SCRIPT-NYA-01")).toBe(true);
  }, 60_000);

  // The digit point, named rather than left to the aggregate. Both halves were
  // falsified before this was kept: a fabricated id fails the "probes only atoms
  // that EXIST" test above AND the per-atom loop below, and nulling either probe
  // fails the coverage total, the Lipi column and this file's own `toEqual`.
  it("closes BN-A1-Q-03 and BN-A1-LIP-11 on one set of eleven atoms", () => {
    const { lessons } = loadEverything();
    const taught = trackIntroducedAtoms(lessons, "bengali");
    const digits = inventory.points.find((point) => point.id === "BN-A1-Q-03");
    const script = inventory.points.find((point) => point.id === "BN-A1-LIP-11");
    // Listed in NUMERICAL order here and taught in no such order: the chapter
    // runs zero, two, three, one, five, six, eight, four, nine, seven -- by what
    // the reader can bring to each shape, with the two false friends last and
    // each of them taught immediately after the Bengali digit whose value its
    // shape suggests.
    const probe = [
      "BN-SCRIPT-DIGIT-ZERO-01",
      "BN-SCRIPT-DIGIT-ONE-01",
      "BN-SCRIPT-DIGIT-TWO-01",
      "BN-SCRIPT-DIGIT-THREE-01",
      "BN-SCRIPT-DIGIT-FOUR-01",
      "BN-SCRIPT-DIGIT-FIVE-01",
      "BN-SCRIPT-DIGIT-SIX-01",
      "BN-SCRIPT-DIGIT-SEVEN-01",
      "BN-SCRIPT-DIGIT-EIGHT-01",
      "BN-SCRIPT-DIGIT-NINE-01",
      // The rule, with BOTH its edges: eight shapes may be trusted, and exactly
      // two may not -- the 8-shape is four and the 9-shape is seven.
      "BN-SCRIPT-DIGIT-FALSE-FRIENDS-01",
    ];
    expect(digits?.probe).toEqual(probe);
    // ONE piece of work filed in two columns, so the two points carry the SAME
    // probe. If they ever diverge, one of the two notes has gone stale.
    expect(script?.probe).toEqual(probe);
    for (const atom of probe) expect(taught.has(atom), atom).toBe(true);
    // WHAT IS NOT CLAIMED, asserted rather than left to be inferred: the WORDS
    // for zero and for six to nine. The reader can read every digit and say only
    // the first five, and no lexical atom for the missing words exists anywhere.
    expect(taught.has("BN-LEX-NUMBERS-ONE-TO-FIVE")).toBe(true);
    for (const absent of [
      "BN-LEX-SHUNYO-01",
      "BN-LEX-CHHOY-01",
      "BN-LEX-SHAT-01",
      "BN-LEX-AAT-01",
      "BN-LEX-NOY-01",
    ]) expect(taught.has(absent), absent).toBe(false);
    // And the cardinal point they would close is still open, which is where the
    // gap is recorded.
    expect(inventory.points.find((point) => point.id === "BN-A1-Q-01")?.probe).toBeNull();
  }, 60_000);
});
