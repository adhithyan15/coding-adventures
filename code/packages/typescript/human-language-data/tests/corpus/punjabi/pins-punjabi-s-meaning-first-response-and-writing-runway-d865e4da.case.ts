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

it("pins Punjabi's meaning-first response and writing runway", () => {
  const opening = loadTrackLessons("punjabi")
    .sort((left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence))
    .slice(0, 10);
  expect(opening.map((lesson) => lesson.realization.lessonId)).toEqual([
    "PA-C01-sat-sri-akal",
    "PA-C01-namaste",
    "PA-C01-han-nahin",
    "PA-W01-ha",
    "PA-W01-aa-matra",
    "PA-W01-bindi",
    "PA-W01-haan-assemble",
    "PA-W01-haan-guided-copy",
    "PA-W01-haan-delayed-copy",
    "PA-W01-haan-dictation",
  ]);
  expect(opening.every((lesson) => lesson.frontmatter.chapter === "1")).toBe(true);

  const meaningFirst = opening[2]!;
  expect(meaningFirst.realization.gloss).toContain("yes / no");
  expect(meaningFirst.frontmatter.skills).toEqual(["listening", "speaking"]);
  expect(meaningFirst.body).toContain("The printed forms are labels, not a reading test.");
  expect(meaningFirst.body).toContain("The next three tiny sessions will teach one\npiece at a time");
});
