import { expect, it } from "vitest";
import { loadEverything, loadExamInventory } from "../../src/loader.js";
import { formatExamCoverage, measureExamCoverage } from "../../src/exam-inventory.js";
import { measureContinuity } from "../../src/continuity.js";
import { defaultCurriculumRoot, loadTrackLessons } from "../../src/loader.js";
import { expectLanguageContinuity, expectLanguageModality } from "./assert-language-corpus.js";
it("pins Telugu continuity", () => expectLanguageContinuity("telugu"));
it("pins Telugu modality", () => expectLanguageModality("telugu"));
it("keeps Telugu's opening free of future farewells and pronouns", () => {
  const references = measureContinuity(
    loadTrackLessons("telugu", defaultCurriculumRoot()),
  ).forwardReferences;
  expect(references.length).toBeLessThanOrEqual(12);
  expect(references.filter((reference) => /-C0[12]-/.test(reference.lessonId))).toEqual([]);
});

// ---------------------------------------------------------------------------
// TELUGU A1 COVERAGE WAS MEASURABLE AND MEASURED BY NOBODY.
//
// `core/exam-inventory-telugu-a1.json` enumerates more points than any other
// inventory in the corpus -- 326 -- and no test loaded it. It was one of three
// (with Hindi's and Sanskrit's) that nothing read, so a tranche could raise the
// number, a retired atom could lower it, and the suite said the same thing
// either way.
//
// Falsified before being kept: a fabricated atom id in the inventory fails the
// census's existence gate, and nulling any covered point's probe fails the
// count below.
// ---------------------------------------------------------------------------
it("pins Telugu A1 exam coverage, which nothing read before", () => {
  const { lessons } = loadEverything();
  const coverage = measureExamCoverage(loadExamInventory("telugu", "A1"), lessons);
  expect(coverage.enumerated).toBe(326);
  expect(coverage.covered).toBe(213);
  expect(coverage.unmapped).toBe(113);
  expect(coverage.partial).toBe(0);
  expect(formatExamCoverage(coverage)).toContain(
    "telugu A1 (partial inventory): 213/326 points covered (65%)",
  );
}, 120_000);
