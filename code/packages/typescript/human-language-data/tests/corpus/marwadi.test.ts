import { expect, it } from "vitest";
import { compileLessonActivities } from "../../src/activity.js";
import { loadTrackLessons } from "../../src/loader.js";
import { measureScriptClosure } from "../../src/script-closure.js";
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
    "delayed-copy",
    "observe-trace",
    "observe-trace",
    "delayed-copy",
    "dictation-transcription",
    "delayed-copy",
    "delayed-copy",
    "delayed-copy",
    "dictation-transcription",
    "delayed-copy",
    "dictation-transcription",
    "delayed-copy",
  ]);
});

it("pins Marwadi-owned chapters and objective activities", () => {
  const lessons = loadTrackLessons("marwadi");
  expect(lessons).toHaveLength(76);
  expect(new Set(lessons.map((lesson) => Number(lesson.frontmatter.chapter)))).toEqual(
    new Set([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]),
  );
  const activities = lessons.flatMap((lesson) => compileLessonActivities(lesson.blocks));
  expect(activities).toHaveLength(76);
  expect(lessons.every((lesson) => compileLessonActivities(lesson.blocks).length === 1)).toBe(true);
  expect(activities.map((activity) => activity.id).sort()).toEqual([
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
    "MW-C06-answer-write",
    "MW-C06-hear-answer-meaning",
    "MW-C06-hear-question-meaning",
    "MW-C06-practice-wellbeing",
    "MW-C06-question-write",
    "MW-C07-hear-later-meaning",
    "MW-C07-practice-later",
    "MW-C07-read-later-build",
    "MW-C08-baap-pair",
    "MW-C08-bahan-siblings",
    "MW-C08-bhai-dictation",
    "MW-C08-dada-dictation",
    "MW-C08-dadi-grandparent-pair",
    "MW-C08-family-four-payoff",
    "MW-C08-family-seven-payoff",
    "MW-C08-hear-baap-meaning",
    "MW-C08-hear-bahan-meaning",
    "MW-C08-hear-bhai-meaning",
    "MW-C08-hear-dada-meaning",
    "MW-C08-hear-dadi-meaning",
    "MW-C08-hear-maa-meaning",
    "MW-C08-hear-parivaar-meaning",
    "MW-C08-maa-dictation",
    "MW-C08-parivaar-dictation",
    "MW-R08-family-foundation-three",
    "MW-R08-family-map-four",
    "MW-R08-script-close-three",
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
    "MW-W06-ttha-read",
    "MW-W06-uu-matra-build",
    "MW-W07-chha-read",
    "MW-W07-e-matra-build",
    "MW-W07-i-matra-order",
    "MW-W07-la-write",
    "MW-W08-ba-write",
    "MW-W08-da-write",
    "MW-W08-va-write",
  ]);

  const closure = measureScriptClosure(lessons);
  expect(closure.violations.filter((violation) => violation.language === "marwadi")).toEqual([]);
  expect(closure.tracks.find((track) => track.language === "marwadi")).toMatchObject({
    lessonCount: 76,
    neverTaughtGlyphs: 0,
    violations: 0,
  });
});
