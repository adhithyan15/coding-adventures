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
  expect(malayalam.validEvidence.find((entry) => entry.lessonId === "ML-W108-a1-timed-production")).toMatchObject({
    lessonId: "ML-W108-a1-timed-production",
    stage: "timed-assessment-production",
  });
  expect(malayalam.levels.find((level) => level.level === "A1")?.missingStages).toEqual([]);
});

it("extends Malayalam's writing runway through A2 connected composition", () => {
  const malayalam = languageWritingStages("malayalam");
  expect(malayalam.defects).toEqual([]);
  expect(malayalam.validEvidence.find((entry) => entry.lessonId === "ML-W338-a2-connected-composition")).toMatchObject({
    lessonId: "ML-W338-a2-connected-composition",
    stage: "connected-composition",
  });
  expect(malayalam.levels.find((level) => level.level === "A2")?.missingStages).toEqual([]);
});

it("extends Malayalam's writing runway into the B1 core with six connected lines", () => {
  // ML-W348-six-lines, the B1 core's writing task (chapter 348): six lines about
  // a trip, with a scene, events in order, a reason, a feeling and an unreal
  // condition. A second connected composition, later in sequence than the A2
  // message, so it is now the last valid evidence on the runway.
  const malayalam = languageWritingStages("malayalam");
  expect(malayalam.defects).toEqual([]);
  expect(malayalam.validEvidence.at(-1)).toMatchObject({
    lessonId: "ML-W348-six-lines",
    stage: "connected-composition",
  });
});
