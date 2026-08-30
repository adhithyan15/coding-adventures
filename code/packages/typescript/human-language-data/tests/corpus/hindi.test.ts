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
    lessons: 281,
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

it("removes support gently from a known phrase to a no-model two-sentence purpose", () => {
  const ordered = loadTrackLessons("hindi").sort(readingOrder);
  const ids = [
    "HI-W06-name-sentence-frame",
    "HI-W06-name-sentence-stop",
    "HI-W06-name-sentence-delayed",
    "HI-W06-name-sentence-dictation",
    "HI-W06-two-sentence-card",
    "HI-W06-two-sentence-no-model",
  ];
  const lessons = ordered.filter((lesson) => ids.includes(lesson.realization.lessonId));

  expect(lessons.map((lesson) => lesson.realization.lessonId)).toEqual(ids);
  expect(lessons.map((lesson) => Number(lesson.frontmatter["duration.max_seconds"]))).toEqual([
    150, 120, 150, 150, 180, 180,
  ]);
  for (let index = 1; index < lessons.length; index += 1) {
    expect(lessons[index]?.frontmatter.prerequisites).toContain(ids[index - 1]);
  }

  const markdown = lessons.map((lesson) =>
    lesson.blocks.map((block) => block.markdown).join("\n"),
  );
  expect(markdown[0]).toContain("four visible word groups");
  expect(markdown[1]).toContain("changes only the sentence boundary");
  expect(markdown[2]).toContain("no visible answer and no romanization");
  expect(markdown[3]).toContain("from sound alone");
  expect(markdown[4]).toContain("new classmate");
  expect(markdown[5]).toContain("There is no Devanagari model and no romanized answer");
  expect(markdown[5]).toContain("two meanings in the requested order");
  expect(markdown[5]).toContain("one **।** after each sentence");
});
