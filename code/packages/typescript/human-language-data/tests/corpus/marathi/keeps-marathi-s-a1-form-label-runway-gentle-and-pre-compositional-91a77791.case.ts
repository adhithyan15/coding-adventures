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

it("keeps Marathi's A1 form-label runway gentle and pre-compositional", () => {
  const ordered = loadTrackLessons("marathi").sort(readingOrder);
  const ids = ordered.filter((lesson) => lesson.realization.chapter === 23);
  expect(ids.map((lesson) => lesson.realization.lessonId)).toEqual([
    "MR-A1F01-naav",
    "MR-A1F01-shahar",
    "MR-A1F01-bhasha",
    "MR-A1F01-avdate-pey",
    "MR-A1F01-avadti-kruti",
    "MR-A1F01-mitrache-naav",
    "MR-A1F02-first-three-copy",
    "MR-A1F02-last-three-copy",
    "MR-A1F03-first-three-delayed",
    "MR-A1F03-last-three-delayed",
  ]);
  expect(ids.every((lesson) => Number(lesson.frontmatter["duration.max_seconds"]) <= 210)).toBe(true);
  expect(ids.every((lesson) => lesson.realization.type === "writing")).toBe(true);
  expect(ids.flatMap((lesson) => lesson.blocks.map((block) => block.writingStage)).filter(Boolean)).toEqual([
    "observe-trace",
    "observe-trace",
    "observe-trace",
    "observe-trace",
    "observe-trace",
    "observe-trace",
    "guided-copy",
    "guided-copy",
    "delayed-copy",
    "delayed-copy",
  ]);
  expect(ids.some((lesson) => lesson.body.includes("controlled-composition"))).toBe(false);
  expect(ids.some((lesson) => lesson.body.includes("timed-assessment-production"))).toBe(false);
});
