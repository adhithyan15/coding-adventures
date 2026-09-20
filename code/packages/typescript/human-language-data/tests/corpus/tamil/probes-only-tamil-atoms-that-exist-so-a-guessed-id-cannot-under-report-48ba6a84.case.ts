import { expect, it } from "vitest";
import { measureContinuity } from "../../../src/continuity.js";
import {
  defaultCurriculumRoot,
  loadChapterPolicy,
  loadEverything,
  loadExamInventory,
  loadTrackLessons,
} from "../../../src/loader.js";
import {
  formatExamCoverage,
  measureExamCoverage,
  trackIntroducedAtoms,
} from "../../../src/exam-inventory.js";
import { measureRamp, readingOrder } from "../../../src/ramp.js";
import { measureScriptClosure } from "../../../src/script-closure.js";
import { expectLanguageContinuity, expectLanguageModality } from "../assert-language-corpus.js";

// ---------------------------------------------------------------------------
// THE TAMIL A1 INVENTORY HAD NO ASSERTION IN THIS FILE -- the hole HL-C354
// found in Telugu and Hindi and told the next reader to look for in the other
// eighteen. Without these two tests the ordinal tranche could land its atoms,
// wire TA-A1-NUM-04's probe, and leave a coverage number that nothing in the
// track's own test file reads. Both halves were falsified before being kept.
// ---------------------------------------------------------------------------
it("probes only Tamil atoms that EXIST, so a guessed id cannot under-report", () => {
  const { lessons } = loadEverything();
  const taught = trackIntroducedAtoms(lessons, "tamil");
  const unknown: string[] = [];
  for (const point of loadExamInventory("tamil", "A1").points) {
    for (const atom of point.probe ?? []) if (!taught.has(atom)) unknown.push(`${point.id}:${atom}`);
  }
  expect(unknown).toEqual([]);
}, 60_000);
