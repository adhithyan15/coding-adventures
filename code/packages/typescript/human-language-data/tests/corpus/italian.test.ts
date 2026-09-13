import { expect, it } from "vitest";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
  languageWritingStages,
} from "./assert-language-corpus.js";
it("pins Italian continuity", () => expectLanguageContinuity("italian"));
it("pins Italian modality", () => expectLanguageModality("italian"));
it("pins Italian lesson-content budgets", () =>
  expectLanguageLessonBudgets("italian", {
    //
    // 142 -> 145: chapter 36, the reading rung -- six words, six lines and a
    // 47-word passage. No new word: every token was checked to occur in a
    // lesson with a lower sequence number.
    //
    // 145 -> 147: the two writing stages Italian did not prove. A delayed copy
    // of `ciao` with the model covered, and a dictation of it from the sound.
    // No new atoms: both practise the three the greeting already introduced.
    lessons: 147,
    idioms: 4,
    senses: 9,
    cultureClaims: 11,
    unitPrefix: "IT",
  }));

it("pins Italian's pre-A1 writing ladder, now complete", () => {
  const italian = languageWritingStages("italian");

  // Was ["observe-trace", "guided-copy"] -- a foothold, and the pin said so.
  // The two stages after it were exactly what Italian's assessment spec named
  // as its writing-ramp gap, and the pre-A1 paper requires both, so the gap was
  // paid rather than left as a backlog note. The ORDER matters and is asserted
  // here rather than the set: `missing-stage-prerequisite` makes a delayed copy
  // invalid unless the tracing and the guided copy come earlier in sequence.
  expect(italian.validEvidence.map((entry) => entry.stage)).toEqual([
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
  ]);
  expect(italian.defects).toEqual([]);
  expect(italian.levels[0]).toMatchObject({
    level: "pre-A1",
    missingStages: [],
    complete: true,
  });
});
