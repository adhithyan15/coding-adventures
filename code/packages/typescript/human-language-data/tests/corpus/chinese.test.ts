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
    // The A2 stage, proved by the A2 core (chapter 157): ZH-W157-reply.
    "connected-composition",
  ]));
});

it("proves both Chinese A1 writing stages, controlled composition first", () => {
  const chinese = languageWritingStages("chinese");
  expect(chinese.levels[1]).toMatchObject({ level: "A1", complete: true, missingStages: [] });
  // ORDER is the point: a timed paper before a controlled one is invalid, so the
  // composition must be the earlier evidence.
  const a1 = chinese.validEvidence.filter((entry) =>
    entry.stage === "controlled-composition" || entry.stage === "timed-assessment-production");
  // The B1 core's one controlled line (chapter 281) comes later still.
  expect(a1.map((entry) => [entry.lessonId, entry.stage])).toEqual([
    ["ZH-W97-my-own-line", "controlled-composition"],
    ["ZH-W98-with-a-clock", "timed-assessment-production"],
    ["ZH-W281-one-reason", "controlled-composition"],
  ]);
});

it("proves the Chinese A2 writing stage after the A1 composition", () => {
  const chinese = languageWritingStages("chinese");
  expect(chinese.levels[2]).toMatchObject({ level: "A2", complete: true, missingStages: [] });
  // The A2 core's reply to a friend's message (chapter 157) is the connected
  // composition: four to six lines for a named reader and a real purpose. It must
  // come after the controlled composition it builds on (ZH-W97-my-own-line).
  const composed = chinese.validEvidence.filter((entry) =>
    entry.stage === "controlled-composition" || entry.stage === "connected-composition");
  // The B1 core (chapters 278-289) adds three more, after the reply: one
  // controlled line with a reason (chapter 281), a four-line note to a hotel desk
  // (chapter 285) and six connected lines to a friend (chapter 289).
  expect(composed.map((entry) => [entry.lessonId, entry.stage])).toEqual([
    ["ZH-W97-my-own-line", "controlled-composition"],
    ["ZH-W157-reply", "connected-composition"],
    ["ZH-W281-one-reason", "controlled-composition"],
    ["ZH-W285-a-note", "connected-composition"],
    ["ZH-W289-six-lines", "connected-composition"],
  ]);
});

it("lays the Chinese B1 compositions after the A2 reply", () => {
  // assessment-policy.json requires no new writing stage at B1, so the B1 core
  // climbs the ladder it has: one controlled line answering why (因为 … 所以 …),
  // then a four-line note for a named reader (the hotel desk), then six lines
  // to a friend that tell a trip in order and end on advice. Each comes after
  // the reply that proved connected composition at A2.
  const chinese = languageWritingStages("chinese");
  const ids = chinese.validEvidence.map((entry) => entry.lessonId);
  const reply = ids.indexOf("ZH-W157-reply");
  expect(reply).toBeGreaterThanOrEqual(0);
  const b1 = ["ZH-W281-one-reason", "ZH-W285-a-note", "ZH-W289-six-lines"].map((id) => ids.indexOf(id));
  expect(b1.every((index) => index > reply)).toBe(true);
  expect([...b1].sort((a, b) => a - b)).toEqual(b1);
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
