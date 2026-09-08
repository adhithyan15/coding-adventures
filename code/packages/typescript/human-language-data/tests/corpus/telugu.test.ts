import { expect, it } from "vitest";
import { measureContinuity } from "../../src/continuity.js";
import {
  defaultCurriculumRoot,
  loadEverything,
  loadExamInventory,
  loadTrackLessons,
} from "../../src/loader.js";
import {
  formatExamCoverage,
  measureExamCoverage,
  trackIntroducedAtoms,
} from "../../src/exam-inventory.js";
import { expectLanguageContinuity, expectLanguageModality } from "./assert-language-corpus.js";
it("pins Telugu continuity", () => expectLanguageContinuity("telugu"));
it("pins Telugu modality", () => expectLanguageModality("telugu"));
it("keeps Telugu's opening free of future farewells and pronouns", () => {
  const references = measureContinuity(
    loadTrackLessons("telugu", defaultCurriculumRoot()),
  ).forwardReferences;
  expect(references.length).toBeLessThanOrEqual(12);
  expect(references.filter((reference) => /-C0[12]-/.test(reference.lessonId))).toEqual([]);
});

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

it("pins Telugu A1 coverage, and the numeral column the ordinal tranche closed", () => {
  const { lessons } = loadEverything();
  const coverage = measureExamCoverage(loadExamInventory("telugu", "A1"), lessons);
  expect(coverage.enumerated).toBe(326);
  expect(coverage.covered).toBe(214);
  expect(coverage.unmapped).toBe(112);
  expect(coverage.partial).toBe(0);
  // HL-C350 measured ordinals as the weakest single column in the corpus --
  // twenty tracks enumerate an ordinal point and eighteen left it uncovered --
  // and Telugu's TE-A1-NUM-04 was the LAST open point in its numeral column,
  // against the highest cardinal ceiling any track reaches. It is now closed by
  // one irregular word (modati) and one ending (-va). The ninth point in this
  // category is TE-A1-NUM-09 and it is a different problem.
  expect(coverage.byCategory["Sankhyalu (numerals and quantity)"]!).toEqual({
    enumerated: 9,
    covered: 8,
  });
  expect(formatExamCoverage(coverage)).toContain(
    "telugu A1 (partial inventory): 214/326 points covered (66%)",
  );
}, 60_000);
