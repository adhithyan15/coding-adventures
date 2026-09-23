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

it("pins Tamil A1 coverage, and the ordinal point the tranche closed", () => {
  const { lessons } = loadEverything();
  const coverage = measureExamCoverage(loadExamInventory("tamil", "A1"), lessons);
  expect(coverage.enumerated).toBe(262);
  // 175 -> 176: TA-A1-PRON-03, the rest of the subject paradigm. THE GAP WAS
  // INSIDE A RULE THAT HAD ALREADY BEEN TAUGHT: TA-C37-ivar teaches the i-/a-
  // pointing system and its table says in as many words that a- points away --
  // and the a- column had never been filled, so a reader could state what a-
  // meant and had no a- person word to say. avar now sits one lesson after
  // ivar, where that table shows the column; avan and aval are the familiar
  // pair ivar's own lesson already named as what it was built from; avarkaL is
  // avar plus the plural -kaL the reader has been pronouncing inside niingaL
  // since chapter two. naam against naangaL is the point the note called out as
  // the one Spanish does not make, and it is taught as a QUESTION -- is my
  // listener inside this 'we' -- rather than as a pair of words.
  // TA-A1-V-06'S NOTE WAS HALF WRONG AND IS CORRECTED IN THE INVENTORY: it said
  // 'no lesson puts a verb into the past', and TA-C32-po prints poogiReen /
  // pooneen / pooveen in a three-row table and glosses pooneen as 'I went'. The
  // real gap is PRODUCTIVITY, not exposure -- shown for one verb, never taught
  // as an atom -- which is a different and cheaper problem than described.
  // 176 -> 177: TA-A1-PRON-04, the accusative -ai on a person. ITS NOTE WAS
  // ACCURATE AND WAS CHECKED BEFORE ANYTHING WAS WRITTEN -- the dative -ukku was
  // the only case this track taught, and the one body mentioning "accusative"
  // was an etymology aside in TA-C26-kaalai, which is not teaching.
  // THE CHAPTER IS A SECOND PIECE OF EVIDENCE FOR A RULE ALREADY TAUGHT rather
  // than a new system: TA-C06-dative-ukku opens by saying English puts a word in
  // front and Tamil sticks a piece on the back, and calls that the biggest
  // structural fact about the language. ennai and unnai then cost nothing new,
  // because they stand on the changed bodies en- and un- that the dative built.
  // THE RATIONAL SPLIT IS TAUGHT AS A CHOICE THAT MEANS SOMETHING: obligatory on
  // a person, and on a thing the difference between tea and THE tea, which is
  // one of the places Tamil does the work an article would do elsewhere.
  // A DRAFT PUT avarai IN THE THIRD LESSON'S HEADWORD and forwardReferences went
  // 8 -> 9: the first lesson teaches that word, so the checker read the first
  // lesson as pointing two lessons ahead. The headword moved to the tea pair the
  // third lesson actually teaches, which is what it should have been.
  // EVERY EXAMPLE KEEPS A FIRST-PERSON SUBJECT, and a draft of this comment got
  // the reason wrong: it said -een was the only person ending the track teaches.
  // Counting says -een 61 and -iirgaL 14, so both are established. It is the
  // THIRD-PERSON -aar that is not -- one occurrence in the whole track -- and
  // these lessons have third-person OBJECTS, so a first-person subject is what
  // keeps each example inside what the reader has. The verbs were also picked so
  // that no example needs the object-verb consonant doubling nothing has taught.
  expect(coverage.covered).toBe(177);
  expect(coverage.unmapped).toBe(85);
  expect(coverage.partial).toBe(0);
  // TA-A1-NUM-04's old note is what the tranche was built on: it recorded that
  // `mutalil` was taught in chapter 62 as a DISCOURSE word and not as an
  // ordinal. `mutalil` is `mutal` + the locative `-il`, so Tamil's one
  // irregular ordinal was already in the learner's mouth, and the first lesson
  // takes the word apart rather than teaching a new one.
  expect(coverage.byCategory["Eṇṇuppeyar (numerals and quantity)"]!).toEqual({
    enumerated: 8,
    covered: 6,
  });
  // THE PRONOUN COLUMN IS WHERE THIS PASS LANDED, so it gets its own pin rather
  // than leaving the move to a total any category could have shifted.
  expect(coverage.byCategory["Pratippeyar (pronouns)"]!).toEqual({
    enumerated: 9,
    covered: 6,
  });
  expect(coverage.points.find((point) => point.id === "TA-A1-PRON-04")!.covered).toBe(true);
  expect(coverage.points.find((point) => point.id === "TA-A1-PRON-06")!.covered).toBe(false);
  expect(formatExamCoverage(coverage)).toContain(
    "tamil A1 (partial inventory): 177/262 points covered (68%)",
  );
}, 60_000);
