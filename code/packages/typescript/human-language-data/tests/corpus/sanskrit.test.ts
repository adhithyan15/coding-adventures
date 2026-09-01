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
    // 236 -> 245: the nine SA-S115..SA-S123 recognition segments (ौ ट ठ ष ख ज उ फ ओ).
    // Each declares zero idioms, senses and culture claims, so only the measured-lesson
    // count moves; the three content totals below are unchanged, which is the check that
    // the segments really are script lessons and not vocabulary wearing a script label.
    lessons: 245,
    idioms: 11,
    senses: 12,
    cultureClaims: 13,
    unitPrefix: "SA",
  }));
