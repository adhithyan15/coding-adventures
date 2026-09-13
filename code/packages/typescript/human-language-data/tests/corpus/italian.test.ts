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
    //
    // 142 -> 145: chapter 36, the reading rung -- six words, six lines and a
    // 47-word passage. No new word: every token was checked to occur in a
    // lesson with a lower sequence number.
    lessons: 145,
    idioms: 4,
    senses: 9,
    cultureClaims: 11,
    unitPrefix: "IT",
  }));

it("pins Italian's current pre-A1 writing foothold", () => {
  const italian = languageWritingStages("italian");
  expect(italian.validEvidence.map((entry) => entry.stage)).toEqual([
    "observe-trace",
    "guided-copy",
  ]);
});
