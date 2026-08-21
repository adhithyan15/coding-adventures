import { expect, it } from "vitest";
import { measureContinuity } from "../../src/continuity.js";
import { defaultCurriculumRoot, loadTrackLessons } from "../../src/loader.js";
import { expectLanguageContinuity, expectLanguageModality } from "./assert-language-corpus.js";
it("pins Malayalam continuity", () => expectLanguageContinuity("malayalam"));
it("pins Malayalam modality", () => expectLanguageModality("malayalam"));
it("keeps Malayalam's opening free of genuine future farewells and pronouns", () => {
  const references = measureContinuity(
    loadTrackLessons("malayalam", defaultCurriculumRoot()),
  ).forwardReferences;
  expect(references.length).toBeLessThanOrEqual(17);
  expect(
    references.filter(
      (reference) =>
        /-C0[12]-/.test(reference.lessonId) &&
        !(reference.lessonId === "ML-C01-athe" && reference.word === "അത്"),
    ),
  ).toEqual([]);
});
