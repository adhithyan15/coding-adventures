import { expect, it } from "vitest";
import {
  expectLanguageContinuity,
  expectLanguageModality,
  languageWritingStages,
} from "../assert-language-corpus.js";

it("pins Malayalam continuity", () => expectLanguageContinuity("malayalam"));
it("pins Malayalam modality", () => expectLanguageModality("malayalam"));

it("gives Malayalam a complete pre-A1 writing runway", () => {
  const malayalam = languageWritingStages("malayalam");
  expect(malayalam.defects).toEqual([]);
  expect(malayalam.levels[0]).toMatchObject({
    level: "pre-A1",
    complete: true,
    missingStages: [],
  });
  expect(
    new Set(
      malayalam.validEvidence
        .filter((entry) => entry.level === "pre-A1")
        .map((entry) => entry.stage),
    ),
  ).toEqual(new Set([
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
  ]));
});

it("extends Malayalam's writing runway through a complete A1 timed paper", () => {
  const malayalam = languageWritingStages("malayalam");
  expect(malayalam.defects).toEqual([]);
  expect(malayalam.validEvidence.at(-1)).toMatchObject({
    lessonId: "ML-W108-a1-timed-production",
    stage: "timed-assessment-production",
  });
  expect(malayalam.levels.find((level) => level.level === "A1")?.missingStages).toEqual([]);
});
