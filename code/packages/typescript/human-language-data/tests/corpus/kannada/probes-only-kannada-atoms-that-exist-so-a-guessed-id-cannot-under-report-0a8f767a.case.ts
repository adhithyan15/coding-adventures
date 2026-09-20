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

// ---------------------------------------------------------------------------
// THE KANNADA A1 INVENTORY HAD NO ASSERTION AT ALL -- the same hole HL-C354
// found in Telugu and Hindi, and the one it told the next reader to go looking
// for in the other eighteen. Without these two tests the ordinal tranche could
// land its atoms, wire KA-A1-NUM-07's probe, and leave a coverage number that
// nothing reads. Both halves were falsified before being kept: a fabricated
// atom id in the probe fails the first, and nulling KA-A1-NUM-07's probe fails
// the second.
// ---------------------------------------------------------------------------
it("probes only Kannada atoms that EXIST, so a guessed id cannot under-report", () => {
  const { lessons } = loadEverything();
  const taught = trackIntroducedAtoms(lessons, "kannada");
  const unknown: string[] = [];
  for (const point of loadExamInventory("kannada", "A1").points) {
    for (const atom of point.probe ?? []) if (!taught.has(atom)) unknown.push(`${point.id}:${atom}`);
  }
  expect(unknown).toEqual([]);
}, 60_000);
