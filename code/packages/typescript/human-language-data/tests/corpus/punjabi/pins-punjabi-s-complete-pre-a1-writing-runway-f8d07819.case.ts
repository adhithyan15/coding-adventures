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

it("pins Punjabi's complete pre-A1 writing runway", () => {
  const punjabi = languageWritingStages("punjabi");
  expect(punjabi.defects).toEqual([]);
  expect(punjabi.levels[0]).toMatchObject({ level: "pre-A1", complete: true, missingStages: [] });
  expect(punjabi.validEvidence.map((entry) => entry.stage)).toEqual([
    "observe-trace",
    "observe-trace",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
    "controlled-composition",
    "controlled-composition",
    "controlled-composition",
    "controlled-composition",
    "controlled-composition",
    "controlled-composition",
    "controlled-composition",
    "controlled-composition",
    "controlled-composition",
    "controlled-composition",
    "guided-copy",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
    "controlled-composition",
    "controlled-composition",
    "guided-copy",
    "guided-copy",
    "guided-copy",
    "guided-copy",
    "guided-copy",
    "guided-copy",
    // PA-W08-digit-recognition: "Hear: zero, one, two … write one digit after
    // each word" is a dictation, not the delayed copy it once declared.
    "dictation-transcription",
    "guided-copy",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
    "controlled-composition",
    "controlled-composition",
    "controlled-composition",
    // Chapters 31-36. Nine single-letter observe/trace sessions, interleaved with the
    // guided and delayed copies that spend each new letter on a word already known by
    // ear. The pattern alternates on purpose: no two assembly steps run back to back.
    "observe-trace",
    "guided-copy",
    "observe-trace",
    "observe-trace",
    "guided-copy",
    "observe-trace",
    "guided-copy",
    "observe-trace",
    "guided-copy",
    "observe-trace",
    "observe-trace",
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "observe-trace",
    "delayed-copy",
    // 51 -> 59. The joining tranche adds eight: PA-W10-thatha is the only new
    // LETTER in seven chapters and contributes an observe-trace and a guided
    // copy, and six word-writing lessons -- nahin, ate, par, ki, je, oh --
    // contribute one guided copy each. Every one of those six spends ZERO new
    // signs.
    "guided-copy",
    // 51 -> 59. The joining tranche adds eight: PA-W10-thatha is the only new
    // LETTER in seven chapters and contributes an observe-trace and a guided
    // copy, and six word-writing lessons -- nahin, ate, par, ki, je, oh --
    // contribute one guided copy each. Every one of those six spends ZERO new
    // signs.
    "guided-copy",
    // 51 -> 59. The joining tranche adds eight: PA-W10-thatha is the only new
    // LETTER in seven chapters and contributes an observe-trace and a guided
    // copy, and six word-writing lessons -- nahin, ate, par, ki, je, oh --
    // contribute one guided copy each. Every one of those six spends ZERO new
    // signs.
    "guided-copy",
    // 51 -> 59. The joining tranche adds eight: PA-W10-thatha is the only new
    // LETTER in seven chapters and contributes an observe-trace and a guided
    // copy, and six word-writing lessons -- nahin, ate, par, ki, je, oh --
    // contribute one guided copy each. Every one of those six spends ZERO new
    // signs.
    "guided-copy",
    // 51 -> 59. The joining tranche adds eight: PA-W10-thatha is the only new
    // LETTER in seven chapters and contributes an observe-trace and a guided
    // copy, and six word-writing lessons -- nahin, ate, par, ki, je, oh --
    // contribute one guided copy each. Every one of those six spends ZERO new
    // signs.
    "guided-copy",
    // 51 -> 59. The joining tranche adds eight: PA-W10-thatha is the only new
    // LETTER in seven chapters and contributes an observe-trace and a guided
    // copy, and six word-writing lessons -- nahin, ate, par, ki, je, oh --
    // contribute one guided copy each. Every one of those six spends ZERO new
    // signs.
    "guided-copy",
    "observe-trace",
    "guided-copy",
    // Chapter 46 introduces the published paper envelope but does not run
    // an impossible twenty-minute assessment inside one short lesson. The
    // first real timed production follows the independent runways below.
    "connected-composition",
    // Chapter 153 adds six supported copies, then one delayed entry and one
    // dictation before three controlled-composition repair/choice prompts.
    "guided-copy",
    "guided-copy",
    "guided-copy",
    "guided-copy",
    "guided-copy",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
    "controlled-composition",
    "controlled-composition",
    "controlled-composition",
    // Chapter 154 joins three already-taught fields: two supported copies,
    // one brief delay, then separate repairs and an independent checkpoint.
    "guided-copy",
    "guided-copy",
    "delayed-copy",
    "controlled-composition",
    "controlled-composition",
    "controlled-composition",
    "controlled-composition",
    // Chapter 155 joins six taught fields: label copying and supported entry,
    // one delay, separate repair, then a no-model six-line attempt.
    "guided-copy",
    "guided-copy",
    "delayed-copy",
    "controlled-composition",
    "controlled-composition",
    "controlled-composition",
    "controlled-composition",
    // Chapter 156 isolates opening, closing, sentence order, and repairs
    // before supported, delayed, and no-model named-reader messages.
    "guided-copy",
    "guided-copy",
    "guided-copy",
    "guided-copy",
    "guided-copy",
    "guided-copy",
    "guided-copy",
    "delayed-copy",
    "guided-copy",
    "guided-copy",
    "guided-copy",
    "guided-copy",
    "guided-copy",
    "guided-copy",
    "guided-copy",
    "delayed-copy",
    "guided-copy",
    "delayed-copy",
    "controlled-composition",
    "controlled-composition",
    // Chapter 157 practises mixed form cues and message pacing before two
    // separate three-minute no-model checkpoints and an untimed repair.
    "guided-copy",
    "timed-assessment-production",
    "controlled-composition",
    "timed-assessment-production",
    "controlled-composition",
    // Chapter 162 revisits three already taught writing moves after a gap.
    "guided-copy",
    "timed-assessment-production",
    "controlled-composition",
    // Chapter 168 uses three unscored guided copies to retrieve old date-form
    // pieces after a long interval; none claims a fresh timed A1 writing pass.
    "guided-copy",
    "guided-copy",
    "guided-copy",
  ]);
});
