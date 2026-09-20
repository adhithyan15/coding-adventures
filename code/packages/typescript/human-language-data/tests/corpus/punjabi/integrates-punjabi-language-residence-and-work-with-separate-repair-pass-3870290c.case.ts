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

it("integrates Punjabi language, residence, and work with separate repair passes", () => {
  const ordered = loadTrackLessons("punjabi")
    .sort((left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence));
  const preparation = ordered.filter((lesson) => lesson.frontmatter.chapter === "22");
  const checkpoint = ordered.filter((lesson) => lesson.frontmatter.chapter === "23");

  expect(preparation.map((lesson) => lesson.realization.lessonId)).toEqual([
    "PA-W06-three-field-labels",
    "PA-W06-three-field-cues",
    "PA-W06-two-field-supported",
    "PA-W06-three-field-supported",
  ]);
  expect(checkpoint.map((lesson) => lesson.realization.lessonId)).toEqual([
    "PA-W06-selection-repair",
    "PA-W06-spelling-repair",
    "PA-W06-spacing-repair",
    "PA-W06-placement-repair",
    "PA-W06-mixed-repair",
    "PA-W06-three-field-no-model",
    "PA-R23-three-no-model-r1",
  ]);
  expect([...preparation, ...checkpoint].every(
    (lesson) => Number(lesson.frontmatter["duration.max_seconds"]) <= 180,
  )).toBe(true);

  const supported = preparation.at(-1)!;
  expect(supported.body).toContain("This is supported entry, not independent writing evidence.");
  expect(supported.blocks.some((block) => block.writingStage !== undefined)).toBe(false);

  const focused = checkpoint.slice(0, 5);
  expect(focused.map((lesson) => lesson.frontmatter["introduces.knowledge"])).toEqual([
    ["PA-FORM-THREE-SELECTION-REPAIR-01"],
    ["PA-FORM-THREE-SPELLING-REPAIR-01"],
    ["PA-FORM-THREE-SPACING-REPAIR-01"],
    ["PA-FORM-THREE-PLACEMENT-REPAIR-01"],
    ["PA-FORM-THREE-MIXED-REPAIR-01"],
  ]);

  const independent = checkpoint.find(
    (lesson) => lesson.realization.lessonId === "PA-W06-three-field-no-model",
  )!;
  expect(independent.blocks.map((block) => block.writingStage).filter(Boolean)).toEqual([
    "controlled-composition",
  ]);
  expect(independent.body).toContain(
    "There is no value bank, support-language label, romanization, or copyable answer below.",
  );
  expect(independent.body).toContain("> A — **ਭਾਸ਼ਾ: __________**");
  expect(independent.body).toContain("> B — **ਰਿਹਾਇਸ਼: __________**");
  expect(independent.body).toContain("> A — **ਕੰਮ: __________**");
  const [activity] = compileLessonActivities(independent.blocks);
  expect(activity?.prompt).not.toContain("ਪੰਜਾਬੀ");
  expect(activity?.prompt).not.toContain("ਸ਼ਹਿਰ");
  expect(activity?.prompt).not.toContain("ਖੇਤੀ");
  expect(activity?.answer).toBe("ਭਾਸ਼ਾ: ਪੰਜਾਬੀ\nਰਿਹਾਇਸ਼: ਸ਼ਹਿਰ\nਕੰਮ: ਖੇਤੀ");
});
