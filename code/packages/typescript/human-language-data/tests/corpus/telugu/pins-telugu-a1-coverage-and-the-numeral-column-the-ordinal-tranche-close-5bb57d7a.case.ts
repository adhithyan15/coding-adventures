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
  // (ba, sha, ssa, pha) and takes that count to 5, and the one after that
  // teaches the five independent vowels and takes it to ZERO.
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
  // TE-A1-L-10 AND TE-A1-L-08 BOTH CLOSED, and the reason is not that the bar
  // moved. 216 -> 217 -> 218: four consonant lessons (ba, ssa, sha, pha) and
  // five independent-vowel lessons (a, aa, ee, ai, o) take untaught-but-used
  // from 9 to 5 to ZERO. EVERY TELUGU CHARACTER THIS TRACK PRINTS ANYWHERE --
  // consonant, vowel letter, vowel sign, digit, mark -- NOW HAS A LESSON. That
  // count was fifteen three passes ago.
  // BOTH CLOSURES ARE AGAINST THE POINTS' OWN LABELS AND NOT AGAINST A FULL
  // ALPHABET CHART. L-10 asks for the consonants as a set a reader can finish a
  // page with; L-08 asks for the independent vowels FOR A VOWEL THAT STARTS A
  // WORD. The letters still without a lesson -- nga, cha, jha, rra, llla, and
  // the vowels ii, uu, vocalic l, oo, au -- have ZERO occurrences across all
  // 365 lesson bodies, so no word the reader says can anchor them, which is the
  // only way this track ever teaches a letter. A point demanding the full chart
  // is a new inventory entry, not either of these reopened; BACKLOG HL-C391
  // carries it.
  // ONE OF THOSE ABSENCES IS NOT A SCRIPT PROBLEM AND IS FILED AS SUCH: `ii` is
  // the everyday Telugu word for "this" before a noun, and the corpus teaches
  // the PRONOUNS idi and adi (TE-C41) and never the adjective pair. HL-C392
  // carries it as lexical work, where it belongs.
  // PLACEMENT FOLLOWS THIS TRACK'S OWN DESIGN, which teaches a letter just after
  // the word that needs it rather than before: each of the six sits one slot
  // after its first use, wedged at content-sequence+1 the way TE-S136 through
  // TE-S139 already are. ttha moved from chapter 16 to 17 because a fifth atom
  // took chapter 16's payoff to 2/5, under the 0.5 floor. TE-S156-script-recall
  // introduces nothing and exists so the last atoms of the chain are revisited.
  expect(coverage.covered).toBe(218);
  expect(coverage.unmapped).toBe(108);
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
  // THE SCRIPT COLUMN IS WHERE THESE TWO PASSES LANDED, so it gets its own pin:
  // 13/23 -> 15/23, and the points that moved are named rather than left to a
  // total that any other category could have shifted.
  expect(coverage.byCategory["Telugu lipi (script and orthography)"]!).toEqual({
    enumerated: 23,
    covered: 15,
  });
  expect(coverage.points.find((point) => point.id === "TE-A1-L-10")!.covered).toBe(true);
  expect(coverage.points.find((point) => point.id === "TE-A1-L-08")!.covered).toBe(true);
  expect(formatExamCoverage(coverage)).toContain(
    "telugu A1 (partial inventory): 218/326 points covered (67%)",
  );
}, 60_000);
