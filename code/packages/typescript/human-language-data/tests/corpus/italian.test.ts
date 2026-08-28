import { expect, it } from "vitest";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
  languageWritingStages,
} from "./assert-language-corpus.js";
it("pins Italian continuity", () => expectLanguageContinuity("italian"));
it("pins Italian modality", () => expectLanguageModality("italian"));
it("pins Italian lesson-content budgets", () =>
  expectLanguageLessonBudgets("italian", {
    lessons: 85,
    idioms: 4,
    senses: 9,
    cultureClaims: 10,
    unitPrefix: "IT",
  }));

it("pins Italian's current pre-A1 writing foothold", () => {
  const italian = languageWritingStages("italian");
  expect(italian.validEvidence.map((entry) => entry.stage)).toEqual([
    "observe-trace",
    "guided-copy",
  ]);
});
