import { it } from "vitest";
import { expectLanguageLessonBudgets } from "../assert-language-corpus.js";

it("pins Punjabi lesson-content budgets", () =>
  expectLanguageLessonBudgets("punjabi", {
    // The helper derives complete lesson measurement from canonical owners.
    // These three totals remain because they are stable semantic inventories.
    idioms: 6,
    senses: 3,
    cultureClaims: 9,
    unitPrefix: "PA",
  }));
