import { expect, it } from "vitest";
import { measureContinuity } from "../../src/continuity.js";
import {
  defaultCurriculumRoot,
  loadEverything,
  loadExamInventory,
  loadTrackLessons,
} from "../../src/loader.js";
import {
  formatExamCoverage,
  measureExamCoverage,
  trackIntroducedAtoms,
} from "../../src/exam-inventory.js";
import { expectLanguageContinuity, expectLanguageModality } from "./assert-language-corpus.js";
it("pins Kannada continuity", () => expectLanguageContinuity("kannada"));
it("pins Kannada modality", () => expectLanguageModality("kannada"));
it("keeps Kannada's opening free of future farewells and pronouns", () => {
  const references = measureContinuity(
    loadTrackLessons("kannada", defaultCurriculumRoot()),
  ).forwardReferences;
  expect(references.length).toBeLessThanOrEqual(15);
  expect(references.filter((reference) => /-C0[12]-/.test(reference.lessonId))).toEqual([]);
});

// ---------------------------------------------------------------------------
// THE KANNADA A1 INVENTORY HAD NO ASSERTION AT ALL -- the same hole HL-C354
// found in Telugu and Hindi, and the one it told the next reader to go looking
// for in the other eighteen. Without these two tests the ordinal tranche could
// land its atoms, wire KA-A1-NUM-07's probe, and leave a coverage number that
// nothing reads. Both halves were falsified before being kept: a fabricated
// atom id in the probe fails the first, and nulling KA-A1-NUM-07's probe fails
// the second.
// ---------------------------------------------------------------------------
it("probes only Kannada atoms that EXIST, so a guessed id cannot under-report", () => {
  const { lessons } = loadEverything();
  const taught = trackIntroducedAtoms(lessons, "kannada");
  const unknown: string[] = [];
  for (const point of loadExamInventory("kannada", "A1").points) {
    for (const atom of point.probe ?? []) if (!taught.has(atom)) unknown.push(`${point.id}:${atom}`);
  }
  expect(unknown).toEqual([]);
}, 60_000);

it("pins Kannada A1 coverage, and the ordinal point the tranche closed", () => {
  const { lessons } = loadEverything();
  const coverage = measureExamCoverage(loadExamInventory("kannada", "A1"), lessons);
  expect(coverage.enumerated).toBe(258);
  expect(coverage.covered).toBe(194);
  expect(coverage.unmapped).toBe(64);
  expect(coverage.partial).toBe(0);
  // KA-A1-NUM-07 was one of the thirteen ordinal points HL-C354 left open, and
  // the one it priced cheapest: Kannada's -aneya has no exceptions, so ten
  // ordinals follow from one ending on cardinals chapter 7 already taught. The
  // eighth point in this category is KA-A1-NUM-08 (measures), which is a
  // vocabulary absence rather than an ordinal one.
  expect(coverage.byCategory["Sankhye (numerals and quantity)"]!).toEqual({
    enumerated: 8,
    covered: 7,
  });
  expect(formatExamCoverage(coverage)).toContain(
    "kannada A1 (partial inventory): 194/258 points covered (75%)",
  );
}, 60_000);
