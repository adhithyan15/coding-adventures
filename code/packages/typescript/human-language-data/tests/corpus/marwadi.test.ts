import { expect, it } from "vitest";
import { compileLessonActivities } from "../../src/activity.js";
import { loadTrackLessons } from "../../src/loader.js";
import {
  expectLanguageContinuity,
  expectLanguageModality,
  languageWritingStages,
} from "./assert-language-corpus.js";

it("pins Marwadi continuity", () => expectLanguageContinuity("marwadi"));
it("pins Marwadi modality", () => expectLanguageModality("marwadi"));

it("pins Marwadi's complete pre-A1 writing ramp", () => {
  const marwadi = languageWritingStages("marwadi");
  expect(marwadi.defects).toEqual([]);
  expect(marwadi.levels[0]).toMatchObject({ level: "pre-A1", complete: true, missingStages: [] });
  expect(marwadi.validEvidence.map((entry) => entry.stage)).toEqual([
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
    "observe-trace",
    "observe-trace",
    "delayed-copy",
    "dictation-transcription",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
  ]);
});

it("pins Marwadi-owned chapters and objective activities", () => {
  const lessons = loadTrackLessons("marwadi");
  expect(new Set(lessons.map((lesson) => Number(lesson.frontmatter.chapter)))).toEqual(
    new Set([1, 2, 3]),
  );
  expect(
    lessons
      .flatMap((lesson) => compileLessonActivities(lesson.blocks))
      .map((activity) => activity.id)
      .sort(),
  ).toEqual([
    "MW-C01-practice-answer",
    "MW-C01-raam-raam-saa-greeting-cue",
    "MW-C02-aabhaar-build",
    "MW-C02-practice-thanks",
    "MW-C03-haan-saa-build",
    "MW-C03-listen-say-choice",
    "MW-C03-practice-yes",
    "MW-W01-aa-matra-change",
    "MW-W01-ra-read",
    "MW-W01-raam-build",
    "MW-W01-sa-choice",
    "MW-W01-saa-build",
    "MW-W02-aa-independent-choice",
    "MW-W02-bha-sound",
    "MW-W03-anusvara-add",
    "MW-W03-ha-read",
  ]);
});
