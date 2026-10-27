import { readFileSync } from "node:fs";
import { join } from "node:path";
import { expect, it } from "vitest";
import { defaultCurriculumRoot } from "../../src/loader.js";
import {
  expectLanguageContinuity,
  expectLanguageModality,
  languageWritingStages,
} from "./assert-language-corpus.js";
import { directedLessonSeconds } from "./timed-lesson-budget.js";
it("pins Chinese continuity", () => expectLanguageContinuity("chinese"));
it("pins Chinese modality", () => expectLanguageModality("chinese"));

it("pins Chinese's complete pre-A1 writing ramp", () => {
  const chinese = languageWritingStages("chinese");
  expect(chinese.defects).toEqual([]);
  expect(chinese.levels[0]).toMatchObject({ level: "pre-A1", complete: true, missingStages: [] });
  // Later vocabulary chapters add legitimate repetitions of the same stages.
  // Pin the complete required set rather than accidentally treating one example
  // per stage as a ceiling on future gentle writing practice.
  expect(new Set(chinese.validEvidence.map((entry) => entry.stage))).toEqual(new Set([
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
    // The two A1 stages, proved by the A1 core (chapters 97 and 98):
    // ZH-W97-my-own-line and ZH-W98-with-a-clock.
    "controlled-composition",
    "timed-assessment-production",
  ]));
});

it("proves both Chinese A1 writing stages, controlled composition first", () => {
  const chinese = languageWritingStages("chinese");
  expect(chinese.levels[1]).toMatchObject({ level: "A1", complete: true, missingStages: [] });
  // ORDER is the point: a timed paper before a controlled one is invalid, so the
  // composition must be the earlier evidence.
  const a1 = chinese.validEvidence.filter((entry) =>
    entry.stage === "controlled-composition" || entry.stage === "timed-assessment-production");
  expect(a1.map((entry) => [entry.lessonId, entry.stage])).toEqual([
    ["ZH-W97-my-own-line", "controlled-composition"],
    ["ZH-W98-with-a-clock", "timed-assessment-production"],
  ]);
});

it("keeps the Chinese timed lesson inside five minutes on its own clock", () => {
  // A three-minute timer for the form and the message, then at most one minute
  // checking: four minutes, under the five-minute lesson cap. The clock is this
  // book's own; no HSK timing is claimed.
  const markdown = readFileSync(
    join(defaultCurriculumRoot(), "chinese", "lessons", "ZH-W98-with-a-clock.md"),
    "utf8",
  );
  expect(directedLessonSeconds(markdown)).toBe(4 * 60);
});
