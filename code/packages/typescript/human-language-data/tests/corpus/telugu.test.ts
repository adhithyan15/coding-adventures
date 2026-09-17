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
import {
  expectLanguageContinuity,
  expectLanguageModality,
  languageWritingStages,
} from "./assert-language-corpus.js";
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
  // 214 -> 216: TE-A1-L-04 (the vowel signs) and TE-A1-L-12 (the retroflex row).
  // FIFTEEN TELUGU CHARACTERS APPEARED IN THE TRACK'S OWN LESSON BODIES WITH
  // NOTHING TEACHING THEM, counted over every Telugu lesson: ma 756, tta 362,
  // a 282, the oo sign 224, ba 180, the o sign 175, sha 126, aa 97, dha 92,
  // ssa 72, ee 52, ai 13, o 12, pha 4, ttha 3. This pass teaches six of them
  // and untaught-but-used falls 15 -> 9; the tranche after it teaches four more
  // (ba, sha, ssa, pha) and takes that count to 5, all five of them independent
  // vowels and all of them TE-A1-L-08's business.
  // THE OFFICIAL CLOSURE METRIC HID ALL OF IT: measureScriptClosure reported
  // telugu neverTaughtGlyphs 0 and taughtGlyphs 64, because it credits a glyph
  // to any script lesson whose BODY contains it (HL-C383). Under the HL-C386
  // rule -- taught only when a script lesson's HEADWORD is a glyph inventory --
  // only 49 were taught. Do not quote neverTaughtGlyphs for this track.
  // TWO OF THE THREE NOTES WERE WRONG AND ARE CORRECTED IN THE INVENTORY:
  // TE-A1-L-10 counted 'ta 292 times' among untaught consonants, and DENTAL ta
  // has been taught since chapter 5 by TE-S01-copy-in-a-word -- the 292-count
  // letter is RETROFLEX ta, the claim having been duplicated from TE-A1-L-12.
  // TE-A1-L-04 called U+0C42 the oo sign; that is the UU sign, taught at
  // chapter 7. The untaught pair was U+0C4A and U+0C4B.
  // TE-A1-L-10 NO LONGER STAYS OPEN, and the reason it closed is not that the
  // bar moved. 216 -> 217: four more consonant lessons (ba, ssa, sha, pha) take
  // untaught-but-used from 9 to 5, and after them EVERY CONSONANT THE CORPUS
  // PRINTS ANYWHERE HAS A LESSON. That is exactly what the point's own label
  // asks for -- the consonant set as something a reader can finish a page with.
  // The five real letters still without one (nga, cha, jha, rra, llla) have
  // ZERO occurrences across all 365 lesson bodies, so no word the reader says
  // can anchor them, which is the only way this track ever teaches a letter.
  // THE CLOSURE IS AGAINST THE LABEL, NOT AGAINST THE FULL VARGA TABLE. A point
  // demanding all thirty-six is a new inventory entry, not this one reopened;
  // BACKLOG HL-C391 carries it. Under-claiming is still the default -- TE-A1-L-08
  // is left open below for five independent vowels the corpus does print.
  // PLACEMENT FOLLOWS THIS TRACK'S OWN DESIGN, which teaches a letter just after
  // the word that needs it rather than before: each of the six sits one slot
  // after its first use, wedged at content-sequence+1 the way TE-S136 through
  // TE-S139 already are. ttha moved from chapter 16 to 17 because a fifth atom
  // took chapter 16's payoff to 2/5, under the 0.5 floor. TE-S156-script-recall
  // introduces nothing and exists so the last atoms of the chain are revisited.
  expect(coverage.covered).toBe(217);
  expect(coverage.unmapped).toBe(109);
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
  // THE SCRIPT COLUMN IS WHERE THIS PASS LANDED, so it gets its own pin: 13/23
  // -> 14/23, and the point that moved is named rather than left to a total that
  // any other category could have shifted.
  expect(coverage.byCategory["Telugu lipi (script and orthography)"]!).toEqual({
    enumerated: 23,
    covered: 14,
  });
  expect(coverage.points.find((point) => point.id === "TE-A1-L-10")!.covered).toBe(true);
  expect(coverage.points.find((point) => point.id === "TE-A1-L-08")!.covered).toBe(false);
  expect(formatExamCoverage(coverage)).toContain(
    "telugu A1 (partial inventory): 217/326 points covered (67%)",
  );
}, 60_000);

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
  expect(track.validEvidence.map((entry) => [entry.lessonId, entry.stage])).toEqual([
    ["TE-S01-letter-ta", "observe-trace"],
    ["TE-S01-copy-in-a-word", "guided-copy"],
    ["TE-S01-delayed-copy", "delayed-copy"],
    ["TE-S01-dictation", "dictation-transcription"],
  ]);
  expect(track.defects).toEqual([]);
  expect(track.levels[0]).toMatchObject({
    level: "pre-A1",
    missingStages: [],
    complete: true,
  });
});
