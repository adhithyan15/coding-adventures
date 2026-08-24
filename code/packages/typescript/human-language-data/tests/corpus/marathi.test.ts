import { expect, it } from "vitest";
import { measureContinuity } from "../../src/continuity.js";
import { loadTrackLessons } from "../../src/loader.js";
import { readingOrder } from "../../src/ramp.js";
import { expectLanguageContinuity, expectLanguageModality } from "./assert-language-corpus.js";

it("pins Marathi continuity", () => expectLanguageContinuity("marathi"));
it("pins Marathi modality", () => expectLanguageModality("marathi"));

it("keeps Marathi's first writing runway in the learner-visible opening", () => {
  const opening = loadTrackLessons("marathi").sort(readingOrder).slice(0, 20);
  expect(opening.map((lesson) => lesson.realization.lessonId)).toEqual([
    "MR-C01-namaskar",
    "MR-W01-ha",
    "MR-W01-o-matra",
    "MR-W01-ho-delayed-copy",
    "MR-W01-ho-dictation",
    "MR-W01-na",
    "MR-W01-aa-matra",
    "MR-W01-ii-matra",
    "MR-W01-ma",
    "MR-W01-sa",
    "MR-W01-ka",
    "MR-W01-virama",
    "MR-W01-ra",
    "MR-W01-namaskar-read",
    "MR-C01-dhanyavad",
    "MR-C01-ho",
    "MR-C01-baram",
    "MR-C01-nahi",
    "MR-C01-yeto",
    "MR-C01-practice",
  ]);
  expect(opening.every((lesson) => lesson.frontmatter.chapter === "1")).toBe(true);
});

it("gives the five Chapter 11 family atoms genuine R1, R2, and R3 retrieval", () => {
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
