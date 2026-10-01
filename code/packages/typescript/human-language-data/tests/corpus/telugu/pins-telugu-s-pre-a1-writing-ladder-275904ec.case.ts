import { expect, it } from "vitest";
import { measureContinuity } from "../../../src/continuity.js";
import {
  defaultCurriculumRoot,
  loadEverything,
  loadExamInventory,
  loadTrackLessons,
} from "../../../src/loader.js";
import {
  formatExamCoverage,
  measureExamCoverage,
  trackIntroducedAtoms,
} from "../../../src/exam-inventory.js";
import {
  expectLanguageContinuity,
  expectLanguageModality,
  languageWritingStages,
} from "../assert-language-corpus.js";

it("pins Telugu's pre-A1 writing ladder", () => {
  const track = languageWritingStages("telugu");

  // The track had NO stage evidence at all -- 49 script lessons and not one
  // writing-stage directive -- so every one of its writing-stage debts read as
  // outstanding while the lessons that could discharge them sat unmarked.
  //
  // The ORDER is asserted rather than the set. `missing-stage-prerequisite`
  // makes a delayed copy invalid unless the tracing and the guided copy come
  // earlier IN SEQUENCE, so a set-equality assertion would pass on a ladder
  // whose rungs are in the wrong order and therefore prove nothing.
  //
  // TE-S150 now opens the source-backed ladder with the greeting word's ma;
  // TE-S151 follows with the thank-you word's dha. TE-S109 then adds
  // familiar-word na. TE-S152 adds tta, followed by
  // TE-S157 with familiar-word ba and TE-S123 with dda, another
  // vocabulary-first retroflex letter. TE-S113 adds familiar-word pa before
  // the first complete ladder. TE-S110 then revisits observe-and-trace with
  // TE-S111 adds source-backed sa after that ladder, followed by TE-S158's
  // source-backed ssa and TE-S110's
  // familiar-word ya and TE-S159's source-backed sha before TE-S114 adds
  // familiar-word la. TE-S116 gives familiar-word da
  // the same source-backed rung, and TE-S117 follows with familiar-word va.
  // TE-S155 revisits the stage for the aspirated partner ttha after the first
  // complete ladder. TE-S128 then adds familiar-word bha; TE-S130 and
  // TE-S132 later revisit the stage for newly introduced tha and nya.
  // The next rung is a SECOND dictation, roughly 1,150 sequence steps after the
  // first. It is here because the ladder proves the stages are reachable, not
  // that each is practised once: `TE-S170` asks the hand to turn *gau* and
  // *gnya* into shapes with nothing on the page to copy, which is the same
  // stage exercised on a harder pair. `TE-S171`, `TE-S172`, and `TE-S173`
  // then return to observe-and-trace for newly introduced rare letters. The
  // `TE-R152` recall closes that last filmstrip with whole-word composition and
  // a timed production check. A rung may repeat; the ORDER assertion below
  // still forbids one arriving before its prerequisite stages.
  expect(track.validEvidence.map((entry) => [entry.lessonId, entry.stage])).toEqual([
    ["TE-S150-letter-ma", "observe-trace"],
    ["TE-S151-letter-dha", "observe-trace"],
    ["TE-S109-letter-na", "observe-trace"],
    ["TE-S152-letter-tta", "observe-trace"],
    ["TE-S157-letter-ba", "observe-trace"],
    ["TE-S06-letter-ra", "observe-trace"],
    ["TE-S123-letter-dda", "observe-trace"],
    ["TE-S113-letter-pa", "observe-trace"],
    ["TE-S01-letter-ta", "observe-trace"],
    ["TE-S01-copy-in-a-word", "guided-copy"],
    ["TE-S01-delayed-copy", "delayed-copy"],
    ["TE-S01-dictation", "dictation-transcription"],
    ["TE-S111-letter-sa", "observe-trace"],
    ["TE-S158-letter-ssa", "observe-trace"],
    ["TE-S110-letter-ya", "observe-trace"],
    ["TE-S159-letter-sha", "observe-trace"],
    ["TE-S114-letter-la", "observe-trace"],
    ["TE-S116-letter-da", "observe-trace"],
    ["TE-S117-letter-va", "observe-trace"],
    ["TE-S155-letter-ttha", "observe-trace"],
    ["TE-S160-letter-pha", "observe-trace"],
    ["TE-S139-letter-ddha", "observe-trace"],
    ["TE-S126-letter-nna", "observe-trace"],
    ["TE-S128-letter-bha", "observe-trace"],
    ["TE-S130-letter-tha", "observe-trace"],
    ["TE-S132-letter-nya", "observe-trace"],
    ["TE-S170-script-dictation-courtesy-letters", "dictation-transcription"],
    ["TE-S171-letter-nga", "observe-trace"],
    ["TE-S172-letter-chha", "observe-trace"],
    ["TE-S173-letter-jha", "observe-trace"],
    ["TE-R152-jhari-recall", "controlled-composition"],
    ["TE-R152-jhari-recall", "timed-assessment-production"],
  ]);
  expect(track.defects).toEqual([]);
  expect(track.levels[0]).toMatchObject({
    level: "pre-A1",
    missingStages: [],
    complete: true,
  });
});
