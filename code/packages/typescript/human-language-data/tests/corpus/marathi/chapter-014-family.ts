import { expect, it } from "vitest";
import { measureContinuity } from "../../../src/continuity.js";
import { loadTrackLessons } from "../../../src/loader.js";
import { readingOrder } from "../../../src/ramp.js";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
  languageWritingStages,
} from "../assert-language-corpus.js";

it("gives the five Chapter 14 family atoms genuine R1, R2, and R3 retrieval", () => {
  const report = measureContinuity(loadTrackLessons("marathi"));
  const repairedAtoms = new Set([
    "MR-LEX-MITRA",
    "MR-ETYMON-MITRA-BIND",
    "MR-GRAMMAR-TATSAMA-BORROWING",
    "MR-LEX-KUTUMB",
    "MR-GRAMMAR-KUTUMB-NEUTER",
  ]);
  expect(report.reinforcement.filter((defect) => repairedAtoms.has(defect.atom))).toEqual([]);
});
