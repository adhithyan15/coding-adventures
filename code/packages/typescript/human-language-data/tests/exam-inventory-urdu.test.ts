// ---------------------------------------------------------------------------
// The Urdu A1 inventory, in its own file.
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

describe("the committed Urdu A1 inventory", () => {
  const inventory = loadExamInventory("urdu", "A1");
  const spanish = loadExamInventory("spanish", "A1");

  it("keeps every point's probe key, and never an empty probe", () => {
    for (const point of inventory.points) {
      expect(point, `${point.id} has no probe key`).toHaveProperty("probe");
      expect(Array.isArray(point.probe) ? point.probe.length : 1, point.id).toBeGreaterThan(0);
    }
  });

  it("probes only atoms that EXIST, so a guessed id cannot under-report", () => {
    const { lessons } = loadEverything();
    const taught = trackIntroducedAtoms(lessons, "urdu");
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
    // Urdu's own machinery — a postposition where Spanish has a preposition, a
    // dative experiencer where Spanish has gustar — is deriving it.
    expect([...dropped].sort()).toEqual(["A1-ART-02", "A1-PRON-07"]);
  });

  it("marks its own points as its own, in both directions", () => {
    for (const point of inventory.points) {
      const cast = point as unknown as { derivedFrom: string[]; urduSpecific?: boolean };
      expect(cast.urduSpecific === true, point.id).toBe(cast.derivedFrom.length === 0);
    }
    const specific = inventory.points.filter(
      (point) => (point as unknown as { derivedFrom: string[] }).derivedFrom.length === 0,
    );
    expect(specific.map((point) => point.id)).toEqual([
      "UR-A1-PST-05", "UR-A1-REG-03", "UR-A1-REG-04", "UR-A1-REG-05",
      "UR-A1-REG-06", "UR-A1-KHT-03",
    ]);
  });

  it("says the A1 task shape is DECLARED and absent, unlike Arabic's", () => {
    // Each `about` has to state its own envelope. Arabic's A1 task shape carries
    // `basis: external` and cites Avant STAMP 4S; Bengali has no task-shapes/ at
    // all. Urdu's assessment.json declares an A1 level and points all four
    // skills at task-shapes/a1.json, which is not on disk — worse than having
    // nothing, because a declaration reads like an envelope until somebody
    // looks. Copying either sibling's sentence here would have been wrong.
    expect(inventory.about).toMatch(/PROJECT-DEFINED EDITORIAL EQUIVALENT, NOT AN EXTERNAL SYLLABUS/);
    expect(inventory.about).toMatch(
      /EXAM ENVELOPE: AN A1 PAPER IS DECLARED AND IS NOT ON DISK/,
    );
    expect(inventory.about).toMatch(/task-shapes\/a1\.json/);
    expect(inventory.about).toMatch(/NOT SEARCHED, BY INSTRUCTION/);
    expect(inventory.source).toMatch(/^PROJECT-DEFINED\./);
    expect(isExamInventoryComplete(inventory)).toBe(false);
    for (const dimension of EXAM_CONTENT_DIMENSIONS) {
      expect(inventory.scope[dimension].status, dimension).toBe("partial");
    }
  });

  it("answers the caveat's two halves — the script and the Perso-Arabic register", () => {
    // tracks.urdu is the only entry in this series that names WHERE a track
    // diverges from its nearest anchor: "the script and the Persian-Arabic
    // register are where the two [Urdu and Hindi] diverge". Both halves are
    // answered by teaching rather than assertion, which is why this is the
    // strongest column in the file.
    expect(inventory.about).toMatch(
      /THE CAVEAT NAMES THE SCRIPT AND THE PERSO-ARABIC REGISTER, AND THOSE ARE THIS TRACK'S TWO BEST COLUMNS/,
    );
    const register = inventory.points.filter((point) => point.category.startsWith("Tarz-e-kalam"));
    expect(register).toHaveLength(6);
    // Five register atoms and two cross-linguistic ones, all covered.
    expect(register.find((point) => point.id === "UR-A1-REG-03")!.probe).toHaveLength(5);
    expect(register.find((point) => point.id === "UR-A1-REG-04")!.probe).toHaveLength(2);
    // And the finding the frontmatter itself yields: Urdu's `register` field
    // carries SEVEN distinct values where Bengali's and Arabic's are uniform,
    // and no atom asserts that vocabulary, so nothing gates it.
    expect(register.find((point) => point.id === "UR-A1-REG-06")!.note).toMatch(
      /seven distinct values/,
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

  it("reports FOUR full columns, a numeral column still at ZERO, and a flattering script closure that is not one", () => {
    const { lessons } = loadEverything();
    const coverage = measureExamCoverage(inventory, lessons);
    expect(coverage.enumerated).toBe(234);
    // 101 -> 116: the negation-and-joining tranche (chapters 19-22). Fifteen
    // items closed fifteen points, and the design constraint was the SCRIPT
    // rather than the vocabulary: every headword was tested against the fifteen
    // letters the ladder has actually taught before it was written, so نہیں,
    // اور, یا, لیکن, کیوں, کون, کیونکہ, کہ, کے لیے, کہنا and آرام سے all landed
    // inside the taught set, while معاف, مہربانی, مگر, جب and بھی did not and
    // are named in romanization beside the letters that block them.
    //
    // 116 -> 130: the oblique tranche (chapters 23-27). Twenty-one items closed
    // fourteen points, and the ratio comes from ONE RULE. UR-A1-PST-05 -- the
    // shape every Urdu noun takes before a postposition -- is described in its
    // own note as "the invisible prerequisite under four of the five points
    // above", and it is taught first, as a rule, before any postposition that
    // needs it. Four postposition points, both demonstrative points and the
    // third-person possessive all stand on it.
    expect(coverage.covered).toBe(130);
    expect(coverage.unmapped).toBe(104);
    expect(coverage.partial).toBe(0);
    // THE PAYOFF, and the column the oblique was worth. Every postposition point
    // in the file now stands on a lesson, including UR-A1-PST-05 itself.
    expect(coverage.byCategory["Huruf-e-jar (postpositions and case)"]!).toEqual({
      enumerated: 6,
      covered: 6,
    });
    // Three more columns close outright, and two of the three were not aimed at:
    // deixis closes because یہ and وہ needed an oblique before they could stand
    // in front of anything, and possession closes because the third person is
    // built out of that same oblique plus the کا the book has had since chapter
    // three.
    expect(coverage.byCategory["Isharat (demonstratives and deixis)"]!).toEqual({
      enumerated: 4,
      covered: 4,
    });
    expect(coverage.byCategory["Milkiyat (possession)"]!).toEqual({
      enumerated: 3,
      covered: 3,
    });
    expect(coverage.byCategory["Jumla (the sentence)"]!).toEqual({
      enumerated: 7,
      covered: 7,
    });
    // The pronoun column, one point off full: only the relative جو is left, and
    // it needs جیم, which the script ladder has not reached.
    expect(coverage.byCategory["Zameer (pronouns)"]!).toEqual({
      enumerated: 9,
      covered: 8,
    });
    // THE HEADLINE. Not one numeral is taught: ایک and دو both return zero
    // occurrences as words (the two raw matches for دو are inside دوست), and
    // there is no digit, no ordinal and no quantifier. Bengali reaches five and
    // Arabic reaches twenty; this track cannot count to one.
    expect(coverage.byCategory["Adad (numerals and quantity)"]!).toEqual({
      enumerated: 5,
      covered: 0,
    });
    // اور ("and") returned ZERO occurrences in Urdu script and appeared exactly
    // once in the whole corpus — romanised, inside a chapter-15 production
    // prompt that asked the learner to SAY a word the book never taught. That
    // line is now quoted inside UR-C20-aur and paid back in its practice.
    // 2 -> 8. The three that remain each name a LETTER rather than an omission:
    // جو and جب need جیم, جب also needs بے, and ایک … دوسرا needs the numerals.
    expect(coverage.byCategory["Rabt (joining and subordination)"]!).toEqual({
      enumerated: 11,
      covered: 8,
    });
    // The negation column goes 1/4 to 3/4. The one that remains, نہ and مت,
    // is blocked twice over: مت belongs in front of an imperative this track
    // still does not teach, and its second letter has no script lesson.
    expect(coverage.byCategory["Nafi (negation)"]!).toEqual({
      enumerated: 4,
      covered: 3,
    });
    // The script column is where this track looks best and measures worst.
    // measureScriptClosure reports only 4 violations — the best of the three —
    // while teaching 15 glyphs of 35 shown, 20 never taught, which is the WORST
    // proportion of the three. The gap is the exposure rule: exposure-only
    // lessons and exempted glyph-instances, bought by
    // headwordsWithoutRomanization being 0.
    //
    // UNCHANGED by the twenty-six lessons of chapters 23-27: violations stay at
    // 4, glyphs taught stay at 15, never-taught stays at 20 and
    // headwordsWithoutRomanization stays at 0. Every one of the twenty-one new
    // headwords is spellable with the fifteen taught letters, which is why this
    // tranche took the oblique and the postpositions rather than the time column
    // — آج, دن, رات, وقت and کب need five letters the ladder has not reached, and
    // a vocabulary tranche cannot buy them.
    expect(coverage.byCategory["Rasm-ul-khat (script and orthography)"]!).toEqual({
      enumerated: 15,
      covered: 9,
    });
    // The other end: the register column the exam-levels caveat asks for.
    expect(coverage.byCategory["Tarz-e-kalam (register, and the Perso-Arabic layer the caveat names)"]!)
      .toEqual({ enumerated: 6, covered: 4 });
    expect(formatExamCoverage(coverage)).toContain(
      "urdu A1 (partial inventory): 130/234 points covered (56%)",
    );
  }, 60_000);
});
