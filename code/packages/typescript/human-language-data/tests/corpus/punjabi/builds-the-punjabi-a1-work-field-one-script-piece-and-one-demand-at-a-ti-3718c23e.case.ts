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

it("builds the Punjabi A1 work field one script piece and one demand at a time", () => {
  const ordered = loadTrackLessons("punjabi")
    .sort((left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence));
  const runway = ordered.filter((lesson) => lesson.frontmatter.chapter === "20");
  const entry = ordered.filter((lesson) => lesson.frontmatter.chapter === "21");

  expect(runway.map((lesson) => lesson.realization.lessonId)).toEqual([
    "PA-W05-ka",
    "PA-W05-work-label",
    "PA-W05-kha",
    "PA-W05-farming",
    // HL-C443: ਮੌਸਮ (mausam) comes before the ੌ lesson, so the sign is
    // written from a word the reader has already said.
    "PA-C20-mausam",
    "PA-W05-au-matra",
    "PA-W05-job",
  ]);
  expect(entry.map((lesson) => lesson.realization.lessonId)).toEqual([
    "PA-W05-work-select",
    "PA-W05-work-supported",
    "PA-W05-work-spelling",
    "PA-W05-work-spacing",
    "PA-W05-work-agreement",
    "PA-W05-work-delayed",
    "PA-W05-work-repair",
    "PA-W05-work-no-model",
  ]);
  expect([...runway, ...entry].every(
    (lesson) => Number(lesson.frontmatter["duration.max_seconds"]) <= 180,
  )).toBe(true);

  const supported = entry.find(
    (lesson) => lesson.realization.lessonId === "PA-W05-work-supported",
  )!;
  expect(supported.body).toContain("This is supported entry, not independent writing evidence.");
  expect(supported.blocks.some((block) => block.writingStage !== undefined)).toBe(false);

  const focusedLessons = [
    "PA-W05-work-spelling",
    "PA-W05-work-spacing",
    "PA-W05-work-agreement",
    "PA-W05-work-repair",
  ].map((id) => entry.find((lesson) => lesson.realization.lessonId === id)!);
  expect(focusedLessons.map((lesson) => lesson.frontmatter["introduces.knowledge"])).toEqual([
    ["PA-FORM-WORK-SPELLING-CHECK-01"],
    ["PA-FORM-WORK-SPACING-01"],
    ["PA-FORM-WORK-AGREEMENT-01"],
    ["PA-FORM-WORK-REPAIR-01"],
  ]);
  expect(focusedLessons[2]!.body).toContain(
    "This checks field-value meaning, not grammatical gender.",
  );

  const independent = entry.at(-1)!;
  expect(independent.blocks.map((block) => block.writingStage).filter(Boolean)).toEqual([
    "controlled-composition",
  ]);
  expect(independent.body).toContain(
    "There is no value bank, support-language label, or romanized answer below.",
  );
  expect(independent.body).toContain("> A — **ਕੰਮ: __________**");
  expect(independent.body).not.toContain("A — **ਖੇਤੀ**");
  const [activity] = compileLessonActivities(independent.blocks);
  expect(activity?.prompt).not.toContain("farming");
  expect(activity?.prompt).not.toContain("ਖੇਤੀ");
  expect(activity?.answer).toBe("ਖੇਤੀ");
});
