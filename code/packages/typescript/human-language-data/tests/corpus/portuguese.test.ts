import { expect, it } from "vitest";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
  languageWritingStages,
} from "./assert-language-corpus.js";
it("pins Portuguese continuity", () => expectLanguageContinuity("portuguese"));
it("pins Portuguese modality", () => expectLanguageModality("portuguese"));
it("pins Portuguese lesson-content budgets", () =>
  expectLanguageLessonBudgets("portuguese", {
    //
    // 113 -> 116: chapter 29, the reading rung -- six words, six lines and a
    // 42-word passage. No new word: every token was checked to occur in a
    // lesson with a lower sequence number.
    // 116 -> 118: the two writing stages Portuguese did not prove. A delayed copy
    // of ola with the model covered, where the accent is the expected miss, and a
    // dictation, where it stops being a mark to remember and becomes one that can
    // be worked out -- the stress is audible, and the accent records it.
    lessons: 118,
    idioms: 7,
    senses: 7,
    cultureClaims: 11,
    unitPrefix: "PT",
  }));

it("pins Portuguese's pre-A1 writing ladder, now complete", () => {
  const portuguese = languageWritingStages("portuguese");

  // Was ["observe-trace", "guided-copy"] -- a foothold, and the pin said so.
  // The two stages after it were exactly what Portuguese's assessment spec
  // named as its writing-ramp gap, and the pre-A1 paper requires both, so the
  // gap was paid rather than left as a backlog note. The ORDER is asserted
  // rather than the set: `missing-stage-prerequisite` makes a delayed copy
  // invalid unless the tracing and the guided copy come earlier in sequence.
  expect(portuguese.validEvidence.map((entry) => entry.stage)).toEqual([
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
  ]);
  expect(portuguese.defects).toEqual([]);
  expect(portuguese.levels[0]).toMatchObject({
    level: "pre-A1",
    missingStages: [],
    complete: true,
  });
});
