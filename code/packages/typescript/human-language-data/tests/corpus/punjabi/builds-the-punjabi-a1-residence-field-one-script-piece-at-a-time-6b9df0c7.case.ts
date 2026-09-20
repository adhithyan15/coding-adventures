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

it("builds the Punjabi A1 residence field one script piece at a time", () => {
  const ordered = loadTrackLessons("punjabi")
    .sort((left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence));
  const runway = ordered.filter((lesson) => lesson.frontmatter.chapter === "18");
  const entry = ordered.filter((lesson) => lesson.frontmatter.chapter === "19");

  expect(runway.map((lesson) => lesson.realization.lessonId)).toEqual([
    "PA-W04-ra",
    "PA-W04-independent-i",
    "PA-W04-residence-label",
    "PA-W04-dda",
    "PA-W04-village",
    "PA-W04-city",
    "PA-R18-wellbeing-r4",
  ]);
  expect(entry.map((lesson) => lesson.realization.lessonId)).toEqual([
    "PA-W04-residence-select",
    "PA-W04-residence-supported",
    "PA-W04-residence-spacing",
    "PA-W04-residence-delayed",
    "PA-W04-residence-repair",
    "PA-W04-residence-no-model",
  ]);
  expect([...runway, ...entry].every(
    (lesson) => Number(lesson.frontmatter["duration.max_seconds"]) <= 180,
  )).toBe(true);

  const supported = entry.find(
    (lesson) => lesson.realization.lessonId === "PA-W04-residence-supported",
  )!;
  expect(supported.body).toContain("This is supported entry, not independent writing evidence.");
  expect(supported.blocks.some((block) => block.writingStage !== undefined)).toBe(false);

  const independent = entry.at(-1)!;
  expect(independent.blocks.map((block) => block.writingStage).filter(Boolean)).toEqual([
    "controlled-composition",
  ]);
  expect(independent.body).toContain(
    "There is no value bank, support-language label, or romanized answer below.",
  );
  expect(independent.body).toContain("> A — **ਰਿਹਾਇਸ਼: __________**");
  expect(independent.body).not.toContain("A — **ਪਿੰਡ**");
  const [activity] = compileLessonActivities(independent.blocks);
  expect(activity?.prompt).not.toContain("village");
  expect(activity?.prompt).not.toContain("ਪਿੰਡ");
  expect(activity?.answer).toBe("ਪਿੰਡ");
});
