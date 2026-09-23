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
// THE TELUGU A1 INVENTORY HAD NO ASSERTION AT ALL, and that is the failure mode
// HL-C350's repairs were told to avoid: landing atoms and wiring probes, while
// the coverage number nothing reads stays whatever it was. A stale pin that
// agrees merges silently. These two tests are the pin, and they were falsified
// before being kept -- a fabricated atom id fails the first, and nulling
// TE-A1-NUM-04's probe fails the second.
// ---------------------------------------------------------------------------
it("probes only Telugu atoms that EXIST, so a guessed id cannot under-report", () => {
  const { lessons } = loadEverything();
  const taught = trackIntroducedAtoms(lessons, "telugu");
  const unknown: string[] = [];
  for (const point of loadExamInventory("telugu", "A1").points) {
    for (const atom of point.probe ?? []) if (!taught.has(atom)) unknown.push(`${point.id}:${atom}`);
  }
  expect(unknown).toEqual([]);
}, 60_000);
