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

it("migrates Punjabi Chapter 2 without inventing Gurmukhi writing credit", () => {
  const chapter = loadTrackLessons("punjabi")
    .sort((left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence))
    .filter((lesson) => lesson.frontmatter.chapter === "2");

  expect(chapter.map((lesson) => lesson.realization.lessonId)).toEqual([
    "PA-C02-naam",
    "PA-C02-mera",
    "PA-C02-hai",
    "PA-C02-mera-naam-hai",
    "PA-C02-tu-tusi",
    "PA-C02-ki",
    "PA-S02-mamma-rara-lava",
    "PA-C02-tuhada-naam-ki-hai",
    "PA-C02-khushi",
    "PA-S02-sassa-tatta-sihari",
    "PA-C02-practice",
  ]);
  expect(chapter.every((lesson) => lesson.frontmatter.schema_version === "2")).toBe(true);
  expect(
    chapter.every(
      (lesson) => Number(lesson.frontmatter["duration.max_seconds"]) <= 240,
    ),
  ).toBe(true);
  expect(chapter.every((lesson) => lesson.frontmatter.skills?.includes("listening"))).toBe(true);
  expect(chapter.every((lesson) => lesson.frontmatter.skills?.includes("speaking"))).toBe(true);
  // The chapter used to have NO writing at all. #13068's recognition runway adds
  // some, and the promise the chapter actually made was about INDEPENDENT
  // writing, so the check gets narrower rather than looser: the only lessons
  // here that touch the hand are the `delivery: script` runway sessions, every
  // one of them keeps the model visible, and no content lesson claims writing.
  for (const lesson of chapter) {
    if (!lesson.frontmatter.skills?.includes("writing")) continue;
    expect(lesson.frontmatter.delivery).toBe("script");
    expect(lesson.realization.lessonId.startsWith("PA-S")).toBe(true);
    expect(lesson.body).toContain("observe-and-trace with the model visible");
    expect(lesson.body).toContain("Nothing in these");
    // no independent-writing stage is claimed anywhere in chapters 2-13
    expect(lesson.body).not.toContain("hl-writing-stage");
  }

  const payoff = chapter.at(-1)!;
  expect(payoff.body).toContain("Independent Gurmukhi reading and writing are **not scored here**");
  expect(payoff.body).toContain("A romanized answer never counts as Gurmukhi writing");
});
