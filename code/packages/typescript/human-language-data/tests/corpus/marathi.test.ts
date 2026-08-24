import { expect, it } from "vitest";
import { measureContinuity } from "../../src/continuity.js";
import { loadTrackLessons } from "../../src/loader.js";
import { readingOrder } from "../../src/ramp.js";
import { expectLanguageContinuity, expectLanguageModality } from "./assert-language-corpus.js";

it("pins Marathi continuity", () => expectLanguageContinuity("marathi"));
it("pins Marathi modality", () => expectLanguageModality("marathi"));

it("splits Marathi's first writing runway at the chapter atom budget", () => {
  const ordered = loadTrackLessons("marathi").sort(readingOrder);
  const opening = ordered.slice(0, 14);
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
  ]);
  expect(opening.every((lesson) => lesson.frontmatter.chapter === "1")).toBe(true);

  const courtesy = ordered.slice(14, 20);
  expect(courtesy.map((lesson) => lesson.realization.lessonId)).toEqual([
    "MR-C01-dhanyavad",
    "MR-C01-ho",
    "MR-C01-baram",
    "MR-C01-nahi",
    "MR-C01-yeto",
    "MR-C01-practice",
  ]);
  expect(courtesy.every((lesson) => lesson.frontmatter.chapter === "2")).toBe(true);

  const chapterSizes = new Map<string, number>();
  for (const lesson of ordered) {
    const chapter = lesson.frontmatter.chapter;
    chapterSizes.set(chapter, (chapterSizes.get(chapter) ?? 0) + 1);
  }
  expect([...chapterSizes.entries()]).toEqual([
    ["1", 14],
    ["2", 6],
    ["3", 9],
    ["4", 6],
    ["5", 6],
    ["6", 5],
    ["7", 2],
    ["8", 6],
    ["9", 4],
    ["10", 4],
    ["11", 4],
    ["12", 6],
    ["13", 4],
    ["14", 1],
    ["15", 12],
  ]);
});

it("gives the five Chapter 12 family atoms genuine R1, R2, and R3 retrieval", () => {
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
