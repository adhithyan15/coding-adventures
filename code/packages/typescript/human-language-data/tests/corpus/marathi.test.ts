import { expect, it } from "vitest";
import { measureContinuity } from "../../src/continuity.js";
import { loadTrackLessons } from "../../src/loader.js";
import { readingOrder } from "../../src/ramp.js";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
  languageWritingStages,
} from "./assert-language-corpus.js";

it("pins Marathi continuity", () => expectLanguageContinuity("marathi"));
it("pins Marathi modality", () => expectLanguageModality("marathi"));
it("pins Marathi lesson-content budgets", () =>
  expectLanguageLessonBudgets("marathi", {
    lessons: 85,
    idioms: 5,
    senses: 4,
    cultureClaims: 7,
    unitPrefix: "MR",
  }));

it("pins Marathi's complete pre-A1 writing ramp", () => {
  const marathi = languageWritingStages("marathi");
  expect(marathi.defects).toEqual([]);
  expect(marathi.levels[0]).toMatchObject({ level: "pre-A1", complete: true, missingStages: [] });
});

it("keeps Marathi's opening script runways below the chapter atom budget", () => {
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

  const firstDoorway = ordered.slice(14, 21);
  expect(firstDoorway.map((lesson) => lesson.realization.lessonId)).toEqual([
    "MR-C01-dhanyavad",
    "MR-W02-visarga",
    "MR-W02-aa-independent",
    "MR-W02-bha",
    "MR-W02-e-matra",
    "MR-W02-anusvara",
    "MR-W02-ta",
  ]);
  expect(firstDoorway.every((lesson) => lesson.frontmatter.chapter === "2")).toBe(true);

  const secondDoorway = ordered.slice(21, 28);
  expect(secondDoorway.map((lesson) => lesson.realization.lessonId)).toEqual([
    "MR-W03-da",
    "MR-W03-dha",
    "MR-W03-ba",
    "MR-W03-ya",
    "MR-W03-lla",
    "MR-W03-va",
    "MR-W03-dhanyavad-write",
  ]);
  expect(secondDoorway.every((lesson) => lesson.frontmatter.chapter === "3")).toBe(true);

  const chapterSizes = new Map<string, number>();
  for (const lesson of ordered) {
    const chapter = lesson.frontmatter.chapter;
    chapterSizes.set(chapter, (chapterSizes.get(chapter) ?? 0) + 1);
  }
  expect([...chapterSizes.entries()]).toEqual([
    ["1", 14],
    ["2", 7],
    ["3", 7],
    ["4", 5],
    ["5", 9],
    ["6", 6],
    ["7", 6],
    ["8", 5],
    ["9", 6],
    ["10", 6],
    ["11", 4],
    ["12", 4],
    ["13", 4],
    ["14", 6],
    ["15", 4],
    ["16", 1],
    ["17", 12],
    ["18", 5],
  ]);
});

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
