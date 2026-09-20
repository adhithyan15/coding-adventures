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

it("builds the Punjabi phone field from introduced pieces to independent Gurmukhi writing", () => {
  const chapter = loadTrackLessons("punjabi")
    .sort((left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence))
    .filter((lesson) => lesson.frontmatter.chapter === "30");

  expect(chapter.map((lesson) => lesson.realization.lessonId)).toEqual([
    "PA-W08-pha",
    "PA-W08-pairin-bindi",
    "PA-W08-hora",
    "PA-W08-phone-label",
    "PA-W08-digit-zero",
    "PA-W08-phone-a",
    "PA-W08-phone-b",
    "PA-W08-digit-recognition",
    "PA-W08-phone-select",
    "PA-W08-phone-supported",
    "PA-W08-phone-grouping",
    "PA-W08-phone-delayed",
    "PA-W08-phone-dictation",
    "PA-W08-phone-repair",
    "PA-W08-phone-no-model",
  ]);
  expect(chapter.every((lesson) => Number(lesson.frontmatter["duration.max_seconds"]) <= 180)).toBe(true);
  expect(chapter.slice(0, 7).map((lesson) => lesson.frontmatter["introduces.knowledge"])).toEqual([
    ["PA-SCRIPT-PHA-01"],
    ["PA-SCRIPT-PAIRIN-BINDI-01"],
    ["PA-SCRIPT-HORA-01"],
    ["PA-FORM-LABEL-PHONE-01"],
    ["PA-SCRIPT-DIGIT-ZERO-01"],
    ["PA-FORM-PHONE-A-01", "PA-FORM-PHONE-DIGIT-ORDER-01"],
    ["PA-FORM-PHONE-B-01"],
  ]);

  const byId = new Map(chapter.map((lesson) => [lesson.realization.lessonId, lesson]));
  expect(byId.get("PA-W08-phone-supported")!.blocks.map((block) => block.writingStage).filter(Boolean))
    .toEqual(["guided-copy"]);
  expect(byId.get("PA-W08-phone-delayed")!.blocks.map((block) => block.writingStage).filter(Boolean))
    .toEqual(["delayed-copy"]);
  expect(byId.get("PA-W08-phone-dictation")!.blocks.map((block) => block.writingStage).filter(Boolean))
    .toEqual(["dictation-transcription"]);

  const independent = byId.get("PA-W08-phone-no-model")!;
  expect(independent.blocks.map((block) => block.writingStage).filter(Boolean)).toEqual([
    "controlled-composition",
    "controlled-composition",
  ]);
  expect(independent.body).toContain("There is no value bank, support-language label,\nLatin-digit version, or copyable Gurmukhi answer below.");
  expect(independent.body).toContain("> ਖ — **ਫ਼ੋਨ: __________**");
  const [activity] = compileLessonActivities(independent.blocks);
  expect(activity?.prompt).not.toContain("੦੨੫ ੧੨੫");
  expect(activity?.prompt).not.toMatch(/[0-9]/);
  expect(activity?.answer).toBe("੦੨੫ ੧੨੫");
});
