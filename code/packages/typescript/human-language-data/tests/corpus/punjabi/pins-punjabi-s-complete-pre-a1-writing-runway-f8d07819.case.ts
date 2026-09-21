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
    "delayed-copy",
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
    // 59 -> 60, and the list stops growing sideways here. Everything above is a
    // pre-A1 stage repeated across the script runways; this one entry is the
    // A1 timed paper, the seventh and last stage in the ramp. Punjabi proved
    // the other six long ago and failed A1 through C2 on this one alone, so a
    // single lesson closes six level-debts.
    "timed-assessment-production",
    "connected-composition",
  ]);
});
