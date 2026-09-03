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
    lessons: 269,
    idioms: 1,
    senses: 5,
    cultureClaims: 27,
    unitPrefix: "GE",
  }));
