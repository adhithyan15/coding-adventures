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

it("builds the Punjabi A1 language field one script piece at a time", () => {
  const ordered = loadTrackLessons("punjabi")
    .sort((left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence));
  const runway = ordered.filter((lesson) => lesson.frontmatter.chapter === "16");
  const entry = ordered.filter((lesson) => lesson.frontmatter.chapter === "17");

  expect(runway.map((lesson) => lesson.realization.lessonId)).toEqual([
    "PA-W03-bha",
    "PA-W03-sha",
    "PA-W03-language-label",
    "PA-W03-pa",
    "PA-W03-tippi",
    "PA-W03-ja",
    "PA-W03-ba",
    "PA-W03-punjabi",
    "PA-W03-sihari",
    "PA-W03-da",
    "PA-W03-hindi",
  ]);
  expect(entry.map((lesson) => lesson.realization.lessonId)).toEqual([
    "PA-W03-language-select",
    "PA-W03-language-supported",
    "PA-W03-language-delayed",
    "PA-W03-language-no-model",
  ]);
  expect([...runway, ...entry].every(
    (lesson) => Number(lesson.frontmatter["duration.max_seconds"]) <= 180,
  )).toBe(true);

  const supported = entry.find(
    (lesson) => lesson.realization.lessonId === "PA-W03-language-supported",
  )!;
  expect(supported.body).toContain("This is supported entry, not independent writing evidence.");
  expect(supported.blocks.some((block) => block.writingStage !== undefined)).toBe(false);

  const independent = entry.at(-1)!;
  expect(independent.blocks.map((block) => block.writingStage).filter(Boolean)).toEqual([
    "controlled-composition",
  ]);
  expect(independent.body).toContain(
    "There is no value bank, support-language label, or\nromanized answer below.",
  );
  expect(independent.body).toContain("> A — **ਭਾਸ਼ਾ: __________**");
  expect(independent.body).not.toContain("A — **ਪੰਜਾਬੀ**");
  const [activity] = compileLessonActivities(independent.blocks);
  expect(activity?.prompt).not.toContain("Punjabi");
  expect(activity?.prompt).not.toContain("ਪੰਜਾਬੀ");
  expect(activity?.answer).toBe("ਪੰਜਾਬੀ");
});
