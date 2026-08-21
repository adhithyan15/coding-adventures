import { expect, it } from "vitest";
import { measureContinuity } from "../../src/continuity.js";
import { defaultCurriculumRoot, loadTrackLessons } from "../../src/loader.js";
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
