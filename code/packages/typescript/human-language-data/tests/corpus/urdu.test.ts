import { it } from "vitest";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
} from "./assert-language-corpus.js";
it("pins Urdu continuity", () => expectLanguageContinuity("urdu"));
it("pins Urdu modality", () => expectLanguageModality("urdu"));
it("pins Urdu lesson-content budgets", () =>
  expectLanguageLessonBudgets("urdu", {
    lessons: 68,
    idioms: 2,
    senses: 4,
    cultureClaims: 4,
    unitPrefix: "UR",
  }));
