import { expect, it } from "vitest";
import { measureContinuity } from "../../src/continuity.js";
import { defaultCurriculumRoot, loadTrackLessons } from "../../src/loader.js";
import { expectLanguageContinuity, expectLanguageModality } from "./assert-language-corpus.js";
it("pins Tamil continuity", () => expectLanguageContinuity("tamil"));
it("pins Tamil modality", () => expectLanguageModality("tamil"));
it("keeps Tamil's opening free of future farewells and pronouns", () => {
  const references = measureContinuity(
    loadTrackLessons("tamil", defaultCurriculumRoot()),
  ).forwardReferences;
  expect(references.length).toBeLessThanOrEqual(13);
  expect(references.filter((reference) => /-C0[12]-/.test(reference.lessonId))).toEqual([]);
});
