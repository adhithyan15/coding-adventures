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
    "observe-trace",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "delayed-copy",
    "guided-copy",
    "dictation-transcription",
    "observe-trace",
    "delayed-copy",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
  ]);
});

it("pins Marwadi-owned chapters and objective activities", () => {
  const lessons = loadTrackLessons("marwadi");
  expect(new Set(lessons.map((lesson) => Number(lesson.frontmatter.chapter)))).toEqual(
    new Set([1, 2, 3, 4, 5, 6]),
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
    "MW-C04-hear-paani-meaning",
    "MW-C04-paani-build",
    "MW-C04-practice-paani",
    "MW-C05-answer-dictation",
    "MW-C05-hear-hai-meaning",
    "MW-C05-hear-kain-meaning",
    "MW-C05-hear-mharo-meaning",
    "MW-C05-hear-naam-meaning",
    "MW-C05-hear-tharo-meaning",
    "MW-C05-kain-delayed",
    "MW-C05-mharo-delayed",
    "MW-C05-naam-delayed",
    "MW-C05-practice-name-exchange",
    "MW-C05-tharo-contrast",
    "MW-W01-aa-matra-change",
    "MW-W01-ra-read",
    "MW-W01-raam-build",
    "MW-W01-sa-choice",
    "MW-W01-saa-build",
    "MW-W02-aa-independent-choice",
    "MW-W02-bha-sound",
    "MW-W03-anusvara-add",
    "MW-W03-ha-read",
    "MW-W04-ii-matra-build",
    "MW-W04-nna-contrast",
    "MW-W04-pa-build-paa",
    "MW-W05-ai-matra-read",
    "MW-W05-ii-independent-choice",
    "MW-W05-ka-read",
    "MW-W05-na-choice",
    "MW-W05-o-matra-read",
    "MW-W05-tha-read",
    "MW-W05-virama-function",
  ]);
});
