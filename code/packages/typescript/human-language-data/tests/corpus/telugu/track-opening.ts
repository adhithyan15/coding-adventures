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
// THE TELUGU A1 INVENTORY HAD NO ASSERTION AT ALL, and that is the failure mode
// HL-C350's repairs were told to avoid: landing atoms and wiring probes, while
// the coverage number nothing reads stays whatever it was. A stale pin that
// agrees merges silently. These two tests are the pin, and they were falsified
// before being kept -- a fabricated atom id fails the first, and nulling
// TE-A1-NUM-04's probe fails the second.
// ---------------------------------------------------------------------------
