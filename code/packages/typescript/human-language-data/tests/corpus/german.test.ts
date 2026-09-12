import { it } from "vitest";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
} from "./assert-language-corpus.js";
it("pins German continuity", () => expectLanguageContinuity("german"));
it("pins German modality", () => expectLanguageModality("german"));
it("pins German lesson-content budgets", () =>
  expectLanguageLessonBudgets("german", {
    //
    // 339 -> 342: chapter 52, the reading rung -- six nouns, six sentences and a
    // 48-word paragraph. No new word: every token was checked to occur in a
    // lesson with a lower sequence number.
    lessons: 342,
    idioms: 1,
    senses: 5,
    cultureClaims: 32,
    unitPrefix: "GE",
  }));
