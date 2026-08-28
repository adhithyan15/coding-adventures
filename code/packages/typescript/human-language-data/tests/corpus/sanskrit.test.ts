import { it } from "vitest";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
} from "./assert-language-corpus.js";
it("pins Sanskrit continuity", () => expectLanguageContinuity("sanskrit"));
it("pins Sanskrit modality", () => expectLanguageModality("sanskrit"));
it("pins Sanskrit lesson-content budgets", () =>
  expectLanguageLessonBudgets("sanskrit", {
    lessons: 236,
    idioms: 11,
    senses: 12,
    cultureClaims: 13,
    unitPrefix: "SA",
  }));
