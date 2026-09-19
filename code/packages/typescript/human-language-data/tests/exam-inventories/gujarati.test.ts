import { describe, expect, it } from "vitest";
import { loadTrackLessons, loadExamInventory } from "../../src/loader.js";
import {
  EXAM_CONTENT_DIMENSIONS,
  measureExamCoverage,
  formatExamCoverage,
  isExamInventoryComplete,
  trackIntroducedAtoms,
} from "../../src/exam-inventory.js";

describe("the committed Gujarati A1 inventory", () => {
  const inventory = loadExamInventory("gujarati", "A1");
  const spanish = loadExamInventory("spanish", "A1");

  it("keeps every point's probe key, and never an empty probe", () => {
    for (const point of inventory.points) {
      expect(point, `${point.id} has no probe key`).toHaveProperty("probe");
      expect(Array.isArray(point.probe) ? point.probe.length : 1, point.id).toBeGreaterThan(0);
    }
  });

  it("probes only atoms that EXIST, so a guessed id cannot under-report", () => {
    const lessons = loadTrackLessons("gujarati");
    const taught = trackIntroducedAtoms(lessons, "gujarati");
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
  });

  it("marks its own points as its own, in both directions", () => {
    for (const point of inventory.points) {
      const cast = point as unknown as { derivedFrom: string[]; gujaratiSpecific?: boolean };
      expect(cast.gujaratiSpecific === true, point.id).toBe(cast.derivedFrom.length === 0);
    }
    const specific = inventory.points.filter(
      (point) => (point as unknown as { derivedFrom: string[] }).derivedFrom.length === 0,
    );
    expect(specific.map((point) => point.id)).toEqual([]);
  });

  it("says an A1 task shape does NOT exist, unlike Punjabi's", () => {
    // The three inventories in this series have three different envelopes and
    // each `about` has to state its own. Malayalam: nothing at all. Punjabi: a
    // checked-in A1 paper and no mocks. Gujarati: a pre-A1 paper only, with
    // `assessment.json` pointing at an `a1.json` and fourteen mocks that are not
    // on disk. Copying a sibling's sentence here would have claimed an envelope
    // this track does not have.
    expect(inventory.about).toMatch(/PROJECT-DEFINED EDITORIAL EQUIVALENT, NOT AN EXTERNAL SYLLABUS/);
    expect(inventory.about).toMatch(
      /EXAM ENVELOPE: A PRE-A1 TASK SHAPE EXISTS AND AN A1 ONE DOES NOT/,
    );
    expect(inventory.about).toMatch(/task-shapes\/a1\.json does not exist/);
    expect(inventory.about).toMatch(/NOT SEARCHED, BY INSTRUCTION/);
    expect(inventory.source).toMatch(/^PROJECT-DEFINED\./);
    expect(isExamInventoryComplete(inventory)).toBe(false);
    for (const dimension of EXAM_CONTENT_DIMENSIONS) {
      expect(inventory.scope[dimension].status, dimension).toBe("partial");
    }
  });

  it("derives its register column from lesson BODIES, because the frontmatter is uniformly neutral", () => {
    // The finding worth protecting. All 228 lessons declare `register: neutral`,
    // so a tool trusting frontmatter would report that this track makes no
    // register distinction at all. It teaches a tu/tame contrast in four coupled
    // places — the pronoun, the copula, the possessive and the farewell — none
    // of which is visible in any frontmatter field.
    expect(inventory.about).toMatch(
      /THE REGISTER COLUMN IS DERIVED FROM LESSON BODIES, NOT FROM FRONTMATTER/,
    );
    const register = inventory.points.filter((point) => point.category.startsWith("Bhaashaashaili"));
    expect(register).toHaveLength(5);
    // Gender is the track's own column and its deepest grammar: five of the
    // corpus's eight grammar atoms are about it, and Gujarati keeps the Sanskrit
    // neuter that Hindi and Punjabi lost.
    expect(inventory.about).toMatch(/GENDER IS THIS TRACK'S OWN COLUMN/);
    const gender = inventory.points.filter((point) => point.category.startsWith("Ling ("));
    expect(gender).toHaveLength(7);
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

  it("reports a joining column that is no longer empty, and the digits that still are", () => {
    const lessons = loadTrackLessons("gujarati");
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage.enumerated).toBe(210);
    // 100 -> 120. Chapters 35-41 answer this file's own uncovered list: seven
    // chapters, five items each, one new item per lesson.
    // 120 -> 121. HL-C359 closes GU-A1-NUM-05, the ordinal point, with chapter
    // 42. Sankhya goes 1/5 to 2/5; the three still open are all the count
    // stopping at five, not the ordinal.
    // 121 -> 122. HL-C361 closes GU-A1-NUM-03, the cardinals six to ten, with
    // chapter 43. ONE point, and the total is the least of what moved: the
    // reachable count doubles (highest numeral 5 -> 10), the track's numeral
    // LESSONS go 1 -> 6, and the ordinal rule stops being a single worked
    // example and becomes productive above the exception list -- none of which
    // a coverage total can see, because every one of them deepens a tick that
    // was already there.
    expect(coverage.covered).toBe(122);
    expect(coverage.unmapped).toBe(88);
    expect(coverage.partial).toBe(0);
    // THE HEADLINE HAS CHANGED, and this assertion is the record of it. It read
    // `covered: 0` and was the starkest finding in the file: `ane` returned ZERO
    // occurrences in 228 lessons, so a learner could not say "tea and milk"
    // although both words were taught on facing pages.
    //
    // The finding under the finding is why it was cheap to fix. TEN of the
    // eleven joining devices needed NO NEW SIGN -- ane, athava, pan, ke, kemke,
    // maate, tethi, jo, jyaare and je are all spelled in glyphs the track taught
    // before chapter 30. This was never a script debt. Nobody had written the
    // words down.
    expect(coverage.byCategory["Jodaan (joining and subordination)"]!).toEqual({
      enumerated: 11,
      covered: 10,
    });
    // The one left open is the distributive, and it needs no new glyph either --
    // it needs a paired correlative, which is a lesson rather than a script step.
    // Gender is unchanged and still the only FULL column in the file.
    expect(coverage.byCategory["Ling (grammatical gender, of which Gujarati has three)"]!).toEqual({
      enumerated: 7,
      covered: 7,
    });
    // Unchanged, and deliberately so. Script closure over the corpus stays exact
    // -- now 44 of 44, because this tranche spent exactly ONE new letter, the
    // `pha` that `maaf karo` required -- while fourteen alphabet letters and ALL
    // TEN DIGITS are still never taught. Reading the first number alone would say
    // the script is finished; the uncovered points in this column are that
    // distinction, and seven chapters of joining did not touch it.
    expect(coverage.byCategory["Lipi (script and orthography)"]!.covered).toBe(9);
    // Unchanged again after chapter 43, and that is the point worth keeping: the
    // cardinal tranche spent exactly one new letter (44 of 44 glyphs taught
    // becomes 45 of 45) and did not touch a single DIGIT. GU-A1-NUM-08 is still
    // zero of ten, and is now the only reading gap left below ten.
    expect(
      inventory.points.find((point) => point.id === "GU-A1-NUM-08")?.probe,
    ).toBeNull();
    // The columns the tranche moved that it was not aiming at: the negator alone
    // closed the can't-say-I-don't-understand function, and the question family
    // closed four at once.
    expect(coverage.byCategory["Nakaar (negation)"]!).toEqual({ enumerated: 4, covered: 3 });
    expect(coverage.byCategory["Prashna (asking questions)"]!).toEqual({ enumerated: 10, covered: 8 });
    expect(formatExamCoverage(coverage)).toContain(
      "gujarati A1 (partial inventory): 122/210 points covered (58%)",
    );
  }, 60_000);

  // The ordinal point, named rather than left to the aggregate. Both halves
  // were falsified before this was kept: a fabricated id fails the "probes only
  // atoms that EXIST" test above, and nulling the probe fails the coverage
  // total two assertions up.
  it("closes GU-A1-NUM-05 on eight atoms, five words and two shapes", () => {
    const lessons = loadTrackLessons("gujarati");
    const taught = trackIntroducedAtoms(lessons, "gujarati");
    const ordinals = inventory.points.find((point) => point.id === "GU-A1-NUM-05");
    expect(ordinals?.probe).toEqual([
      // The four the language HANDS DOWN, in the order the chapter teaches
      // them -- which runs by how much of the cardinal survives, not by number.
      "GU-LEX-PAHELU",
      "GU-LEX-BIJU",
      "GU-LEX-TRIJU",
      "GU-LEX-CHOTHU",
      // The one it BUILDS, and the suffix that builds it. This is the lesson
      // the chapter OPENS on, because the four above are exceptions to it.
      "GU-LEX-PANCHMU",
      "GU-GRAMMAR-ORDINAL-MU",
      // The shape beejun and treejun share, claimed only once there are two of
      // them: one word is not a shape.
      "GU-GRAMMAR-ORDINAL-IJU",
    ]);
    for (const atom of ordinals?.probe ?? []) expect(taught.has(atom), atom).toBe(true);
    // The cardinal the rule stands on is taught, and is the reason the chapter
    // opens where it does.
    expect(taught.has("GU-LEX-NUMBERS-ONE-TO-FIVE")).toBe(true);
  }, 60_000);

  // The cardinal point, named rather than left to the aggregate. Both halves
  // were falsified before this was kept: a fabricated id fails the "probes only
  // atoms that EXIST" test above AND the per-atom loop below, and nulling the
  // probe fails the coverage total two tests up as well as this file's own
  // `toEqual` on the probe list.
  it("closes GU-A1-NUM-03 on five cardinals, one letter and a rule with both edges", () => {
    const lessons = loadTrackLessons("gujarati");
    const taught = trackIntroducedAtoms(lessons, "gujarati");
    const cardinals = inventory.points.find((point) => point.id === "GU-A1-NUM-03");
    // In COUNTING order, because for six to ten that IS the construction order:
    // Wiktionary's -mun entry ends its exception list at chha, so the reader
    // crosses the boundary between exception and rule exactly once, between the
    // first lesson and the second. Ordering by cost was available -- four of the
    // five need no new sign -- and was rejected, because it buys nothing and
    // leaves the reader counting round a hole at eight.
    expect(cardinals?.probe).toEqual([
      "GU-LEX-CHHA-SIX",
      "GU-LEX-SAAT",
      "GU-LEX-AATH",
      "GU-LEX-NAV",
      "GU-LEX-DAS",
    ]);
    for (const atom of cardinals?.probe ?? []) expect(taught.has(atom), atom).toBe(true);
    // The chapter's other two atoms are NOT in the probe, because neither is a
    // cardinal: the one new letter, and the rule that names both edges of the
    // -mun exception list. Both are taught, and the rule atom is the reason the
    // ordinals above sixth need no lesson of their own.
    expect(taught.has("GU-SCRIPT-TTHA-01")).toBe(true);
    expect(taught.has("GU-GRAMMAR-ORDINAL-MU-REACH")).toBe(true);
    // What is NOT claimed, asserted rather than left to be inferred: the
    // irregular sixth ordinal is printed once with its source and taught as no
    // atom at all, so no GU-LEX-* id for it exists anywhere in the corpus.
    expect(taught.has("GU-LEX-CHHATHTHU")).toBe(false);
    // And the count still stops at ten. GU-A1-NUM-04 wants agiyaar upward.
    expect(inventory.points.find((point) => point.id === "GU-A1-NUM-04")?.probe).toBeNull();
  }, 60_000);
});

// ---------------------------------------------------------------------------
// The first inventory for a track whose exam-levels entry carries NO CAVEAT.
//
// Every earlier file in this series had an external steer to answer. Japanese
// is told JLPT tests no production; Chinese that its CEFR correspondence is
// unpublished; Tamil that it is diglossic; Sanskrit that a traditional
// syllabus is not parallel to CEFR. Russian's entry says exam TORFL, basis
// published, mapping A1 to TEU — and stops.
//
// So the columns this file opens beyond the Spanish set had to be MEASURED
// into existence rather than imported. Walking the proxy's five noun points,
// eight pronoun points and six verb-phrase points against the corpus showed
// that every one of them resolved to a question about CASE, so case is a
// category here rather than a footnote; the same walk over the verb column
// resolved to ASPECT. Ten of the file's fourteen russianSpecific grammar
// points live in those two categories, and neither has any Spanish column.
//
// Two results are worth pinning as SHAPE rather than as size:
//
//   1. The joining column was 0 of 13 when this file was written — the sixth
//      track running, and the first that is neither Indo-Aryan nor Dravidian.
//      `i` ("and") was PRINTED as a conjunction in two lesson bodies and
//      introduced by no lesson at all, which was a hair better than Gujarati's
//      `ane` at zero occurrences and worse in one way: the word was on the page
//      doing work the reader was never told about.
//   2. The repair column was HALF closed, which no percentage would show. The
//      track taught `ya ne ponimayu` and `ya ne znayu` in full, and had no
//      word for sorry and no way to ask for a repeat: `izvinite`, `prostite`,
//      `povtorite` and `medlenno` were each zero across all 88 files.
//
// BOTH ARE NOW CLOSED, by chapters 16-22 (35 lessons, 57 atoms). Thirteen of
// thirteen joining devices are taught and the track can produce a two-clause
// sentence for the first time; the repair kit exists. The assertions below are
// re-pinned at the new figures and the shape claims are kept, in the past
// tense, because the finding they record is about how this corpus was authored
// rather than about where it stands today.
// ---------------------------------------------------------------------------
