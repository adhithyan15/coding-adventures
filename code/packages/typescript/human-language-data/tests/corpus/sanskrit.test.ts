import { expect, it } from "vitest";
import { loadEverything, loadExamInventory } from "../../src/loader.js";
import { formatExamCoverage, measureExamCoverage } from "../../src/exam-inventory.js";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
} from "./assert-language-corpus.js";
it("pins Sanskrit continuity", () => expectLanguageContinuity("sanskrit"));
it("pins Sanskrit modality", () => expectLanguageModality("sanskrit"));
it("pins Sanskrit lesson-content budgets", () =>
  expectLanguageLessonBudgets("sanskrit", {
    // 245 -> 270: the script ladder was rebuilt and moved earlier. Twenty-five new
    // recognition segments (SA-S200..SA-S224) join the twenty-three that already
    // existed, and every one of the forty-eight now credits EXACTLY ONE new
    // Devanagari character, scheduled so the character lands before the first lesson
    // that asks the reader to decode it.
    //
    // Each declares zero idioms, senses and culture claims, so only the measured-lesson
    // count moves; the three content totals below are unchanged, which is the check that
    // the segments really are script lessons and not vocabulary wearing a script label.
    //
    // 270 -> 304: chapters 1-5 left the hand-written set, so their thirty lessons
    // migrated to schema v2 and became measurable at all (the track now has zero
    // measurement-blind lessons); three lessons that each packed two headwords
    // split in two — yes/no, the two words for "you", and "well" versus the reply
    // "I am well"; and chapter 2 gained the recap lesson it never had, so the
    // closing exchange the hand-written chapter carried is still in the book.
    // The three content totals below are again unchanged: the split halves, the
    // migrated lessons and the new recap declare no new idioms, senses or culture
    // claims, so nothing was smuggled in under cover of the migration.
    //
    // 304 -> 335: the clause-joining and past-tense tranche, chapters 52-61.
    // Thirty-one lessons: four joining particles and a review; the ya- / ta-
    // correlative; iti and the clause it makes into an object; the -tva gerund
    // and the -tum infinitive; the imperfect; can and want; the dative-experiencer
    // liking frame; the origin words; the danda; and rtu with the independent ऋ
    // it finally gives a headword to. Re-measured against the tree.
    //
    // The three content totals below are unchanged, and that is the check worth
    // making here: a grammar tranche should declare no new idioms, senses or
    // culture claims, and this one declares none.
    //
    // 335 -> 347: the ordinal tranche, chapters 62-63. Twelve lessons -- ten
    // ordinals one word a lesson, plus each chapter's retrieval payoff -- and
    // thirteen atoms, because three of the twelve carry a naming atom for an
    // ENDING beside the word that shows it. Re-measured against the tree.
    // Idioms, senses and culture claims are again unchanged at 11 / 12 / 13, and
    // here that is a deliberate claim rather than an accident: dvitiyah also
    // means "a companion" in Macdonell and the lesson says so on the page, but
    // no SENSE atom is declared for it, because the tranche teaches the numeral
    // and does not drill the second reading.
    lessons: 347,
    idioms: 11,
    senses: 12,
    cultureClaims: 13,
    unitPrefix: "SA",
  }));

// ---------------------------------------------------------------------------
// SANSKRIT A1 COVERAGE WAS MEASURABLE AND MEASURED BY NOBODY.
//
// `core/exam-inventory-sanskrit-a1.json` has been committed for as long as the
// Dravidian and Indo-Aryan inventories have, and every one of those is pinned
// by a test that fails when its number moves. Sanskrit's was not: it was the
// third of three files -- with Hindi's and Telugu's -- that no test loaded at
// all. A tranche could raise it, a retired atom could lower it, and the suite
// would have said the same thing either way, which is the exact failure mode
// the probe design in `exam-inventory.ts` exists to rule out. A number nothing
// reads is not a measurement.
//
// This is the pin. It was falsified before being kept: fabricating an atom id
// in the file fails the census's existence gate, and nulling any covered
// point's probe fails the count below.
// ---------------------------------------------------------------------------
it("pins Sanskrit A1 exam coverage, which nothing read before", () => {
  const { lessons } = loadEverything();
  const coverage = measureExamCoverage(loadExamInventory("sanskrit", "A1"), lessons);
  expect(coverage.enumerated).toBe(164);
  // 141 -> 142: the ordinal tranche closes SA-A1-Q-02, and with it the Sankhya
  // column outright. One point for twelve lessons is a poor ratio and it is the
  // honest one -- the ordinals unlock nothing else in this inventory. In
  // particular they do NOT unlock SA-A1-SP-02, which SA-A1-Q-02's own note used
  // to claim they would; that point is relative position (left, right, above,
  // below, in front, behind) and it is blocked on six direction words nobody
  // teaches. The note was corrected in the same commit.
  expect(coverage.covered).toBe(142);
  expect(coverage.unmapped).toBe(22);
  expect(coverage.partial).toBe(0);
  // The highest percentage in the corpus, and the reason is worth stating so it
  // is not read as the track being furthest along: this inventory is DERIVED
  // from the Spanish A1 point set and drops what Sanskrit has not got, so its
  // denominator is the smallest of the twenty-five. 86% of 164 is not
  // comparable with 84% of 273.
  expect(formatExamCoverage(coverage)).toContain(
    "sanskrit A1 (partial inventory): 142/164 points covered (87%)",
  );
  // Where the 23 gaps actually are, named rather than counted. Two columns hold
  // 13 of them, and they are the two an editorially derived A1 inventory is
  // bound to strain on: the revival vocabulary a classical corpus never had,
  // and the orthography beyond the characters the script ladder teaches.
  expect(coverage.byCategory[
    "Modern life — fields a spoken revival needs and a classical corpus lacks"
  ]!).toEqual({ enumerated: 15, covered: 5 });
  expect(coverage.byCategory["Lipi — the script and its orthography"]!).toEqual({
    enumerated: 10,
    covered: 7,
  });
  // And the two columns the clause-joining tranche finished outright, which are
  // what a reader of a classical text needs first.
  expect(coverage.byCategory["Samuccaya — coordination"]!).toEqual({ enumerated: 5, covered: 5 });
  expect(coverage.byCategory["Vibhakti — the case system"]!).toEqual({ enumerated: 5, covered: 5 });
  // And the column the ordinal tranche finished: number and quantity, 4/5 -> 5/5.
  expect(coverage.byCategory["Saṅkhyā — number and quantity"]!).toEqual({ enumerated: 5, covered: 5 });
  // The ordinal point, named rather than left to the aggregate, so a nulled
  // probe or a fabricated id is caught HERE and not only by the total. Both
  // halves were falsified before this was kept.
  const ordinals = loadExamInventory("sanskrit", "A1").points.find(
    (point) => point.id === "SA-A1-Q-02",
  );
  expect(ordinals?.probe).toEqual([
    // The productive ending, and the five numbers it makes (chapter 62).
    "SA-LEX-C62-ORDINAL-01",
    "SA-GRAMMAR-C62-ORDINAL-MA-02",
    "SA-LEX-C62-ORDINAL-03",
    "SA-LEX-C62-ORDINAL-04",
    "SA-LEX-C62-ORDINAL-05",
    "SA-LEX-C62-ORDINAL-06",
    // The two endings before it, and the word that uses neither (chapter 63).
    "SA-LEX-C63-ORDINAL-01",
    "SA-GRAMMAR-C63-ORDINAL-THA-02",
    "SA-LEX-C63-ORDINAL-03",
    "SA-LEX-C63-ORDINAL-04",
    "SA-GRAMMAR-C63-ORDINAL-TIYA-05",
    "SA-LEX-C63-ORDINAL-06",
    "SA-LEX-C63-ORDINAL-07",
  ]);
  for (const atom of ordinals?.probe ?? []) {
    expect(coverage.points.find((point) => point.id === "SA-A1-Q-02")?.missingAtoms, atom).toEqual(
      [],
    );
  }
}, 120_000);
