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
    // 68 -> 88: HL-C240, chapters 17 and 18. Twenty schema-v2 lessons that
    // interleave six Nastaliq letters with six new pre-A1 headwords, so the
    // ladder is gloss-first and never a block of alphabet. Idioms, senses and
    // culture claims are unchanged: none of the twenty declares one.
    lessons: 88,
    idioms: 2,
    senses: 4,
    cultureClaims: 4,
    unitPrefix: "UR",
  }));
