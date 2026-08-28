import { expect, it } from "vitest";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
  languageWritingStages,
} from "./assert-language-corpus.js";
it("pins Portuguese continuity", () => expectLanguageContinuity("portuguese"));
it("pins Portuguese modality", () => expectLanguageModality("portuguese"));
it("pins Portuguese lesson-content budgets", () =>
  expectLanguageLessonBudgets("portuguese", {
    lessons: 94,
    idioms: 7,
    senses: 7,
    cultureClaims: 10,
    unitPrefix: "PT",
  }));

it("pins Portuguese's current pre-A1 writing foothold", () => {
  const portuguese = languageWritingStages("portuguese");
  expect(portuguese.validEvidence.map((entry) => entry.stage)).toEqual([
    "observe-trace",
    "guided-copy",
  ]);
});
