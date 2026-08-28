import { expect, it } from "vitest";
import { loadTrackLessons } from "../../src/loader.js";
import { readingOrder } from "../../src/ramp.js";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
  languageWritingStages,
} from "./assert-language-corpus.js";
it("pins Hindi continuity", () => expectLanguageContinuity("hindi"));
it("pins Hindi modality", () => expectLanguageModality("hindi"));
it("pins Hindi lesson-content budgets", () =>
  expectLanguageLessonBudgets("hindi", {
    lessons: 275,
    idioms: 21,
    senses: 22,
    cultureClaims: 27,
    unitPrefix: "HI",
  }));

it("pins Hindi's first cumulative pre-A1 writing-stage runway", () => {
  const hindi = languageWritingStages("hindi");
  expect(hindi.defects).toEqual([]);
  expect(hindi.levels[0]).toMatchObject({ level: "pre-A1", complete: true, missingStages: [] });
});

it("removes support gently from a visible glyph trace to one heard known word", () => {
  const ordered = loadTrackLessons("hindi").sort(readingOrder);
  const stageLessons = ordered.filter((lesson) =>
    [
      "HI-W01-shirorekha-na-ma",
      "HI-W01-na-ma",
      "HI-W05-namaste-delayed-copy",
      "HI-W05-namaste-dictation",
    ].includes(lesson.realization.lessonId),
  );

  expect(stageLessons.map((lesson) => lesson.realization.lessonId)).toEqual([
    "HI-W01-shirorekha-na-ma",
    "HI-W01-na-ma",
    "HI-W05-namaste-delayed-copy",
    "HI-W05-namaste-dictation",
  ]);
  expect(stageLessons.map((lesson) => Number(lesson.frontmatter["duration.max_seconds"]))).toEqual([
    257, 186, 120, 120,
  ]);
  expect(stageLessons[2]?.frontmatter.prerequisites).toContain("HI-W05-write-namaste");
  expect(stageLessons[3]?.frontmatter.prerequisites).toContain("HI-W05-namaste-delayed-copy");
});
