import { expect, it } from "vitest";
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
