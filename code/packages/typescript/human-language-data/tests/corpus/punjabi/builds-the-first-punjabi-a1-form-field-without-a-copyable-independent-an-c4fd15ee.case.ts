import { readFileSync } from "node:fs";
import { join } from "node:path";
import { expect, it } from "vitest";
import { compileLessonActivities } from "../../../src/activity.js";
import { measureContinuity } from "../../../src/continuity.js";
import {
  DOC_SHARD_PLANS,
  defaultRepoRoot,
  unshardDocContents,
} from "../../../src/doc-shard-cli.js";
import { defaultCurriculumRoot, loadTrackLessons } from "../../../src/loader.js";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
  languageWritingStages,
} from "../assert-language-corpus.js";

it("builds the first Punjabi A1 form field without a copyable independent answer", () => {
  const chapter = loadTrackLessons("punjabi")
    .sort((left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence))
    .filter((lesson) => lesson.frontmatter.chapter === "15");

  expect(chapter.map((lesson) => lesson.realization.lessonId)).toEqual([
    "PA-W02-a",
    "PA-W02-aman",
    "PA-W02-manan",
    "PA-W02-name-label",
    "PA-W02-name-select",
    "PA-W02-name-supported",
    "PA-W02-name-delayed",
    "PA-W02-name-no-model",
  ]);
  expect(chapter.every((lesson) => Number(lesson.frontmatter["duration.max_seconds"]) <= 180)).toBe(true);

  const supported = chapter.find((lesson) => lesson.realization.lessonId === "PA-W02-name-supported")!;
  expect(supported.body).toContain("This is supported entry, not independent writing evidence.");
  expect(supported.blocks.some((block) => block.writingStage !== undefined)).toBe(false);

  const independent = chapter.at(-1)!;
  expect(independent.blocks.map((block) => block.writingStage).filter(Boolean)).toEqual([
    "controlled-composition",
  ]);
  expect(independent.body).toContain("There is no value bank, support-language name, or romanized answer below.");
  expect(independent.body).toContain("> A — **ਨਾਂ: __________**");
  expect(independent.body).not.toContain("A ਅਮਨ");
  const [activity] = compileLessonActivities(independent.blocks);
  expect(activity?.prompt).not.toContain("Aman");
  expect(activity?.prompt).not.toContain("ਅਮਨ");
  expect(activity?.answer).toBe("ਅਮਨ");
});
