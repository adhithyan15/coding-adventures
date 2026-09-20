import { it } from "vitest";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
} from "../assert-language-corpus.js";

it("pins Hindi continuity", () => expectLanguageContinuity("hindi"));
it("pins Hindi modality", () => expectLanguageModality("hindi"));
it("pins Hindi lesson-content budgets", () =>
  expectLanguageLessonBudgets("hindi", { unitPrefix: "HI" }));
