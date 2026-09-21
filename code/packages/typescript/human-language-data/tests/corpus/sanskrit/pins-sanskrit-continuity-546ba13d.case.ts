import { expect, it } from "vitest";
import { loadEverything, loadExamInventory } from "../../../src/loader.js";
import { formatExamCoverage, measureExamCoverage } from "../../../src/exam-inventory.js";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
  languageWritingStages,
} from "../assert-language-corpus.js";

it("pins Sanskrit continuity", () => expectLanguageContinuity("sanskrit"));
