import { expect, it } from "vitest";
import { measureContinuity } from "../../../src/continuity.js";
import {
  defaultCurriculumRoot,
  loadEverything,
  loadExamInventory,
  loadTrackLessons,
} from "../../../src/loader.js";
import {
  formatExamCoverage,
  measureExamCoverage,
  trackIntroducedAtoms,
} from "../../../src/exam-inventory.js";
import {
  expectLanguageContinuity,
  expectLanguageModality,
  languageWritingStages,
} from "../assert-language-corpus.js";
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
