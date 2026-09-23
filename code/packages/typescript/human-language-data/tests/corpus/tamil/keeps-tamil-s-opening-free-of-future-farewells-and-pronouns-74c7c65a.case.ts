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

it("keeps Tamil's opening free of future farewells and pronouns", () => {
  const references = measureContinuity(
    loadTrackLessons("tamil", defaultCurriculumRoot()),
  ).forwardReferences;
  // THE CAP MOVED 7 -> 8 AND THE DEBT DID NOT. Chapter 37 teaches avar one
  // lesson after ivar, so ivar's own near/far table -- which prints avar to
  // show what the a- front letter does -- became a one-lesson-early reference
  // to it. That is this track's ESTABLISHED PATTERN for a near/far pair, not
  // new debt: TA-C40 already carries three of them (here/there, this/that,
  // who/where), all at exactly one lesson early. avar could only avoid it by
  // being taught BEFORE ivar, which would put the far cell in front of the
  // anchor that teaches the pointing system.
  // So the total keeps a ceiling and the assertion that carries the meaning is
  // the one below it: references at a real DISTANCE have not grown.
  expect(references.length).toBeLessThanOrEqual(8);
  expect(references.filter((reference) => reference.lessonsEarly > 1).length)
    .toBeLessThanOrEqual(4);
  expect(references.filter((reference) => /-C0[12]-/.test(reference.lessonId))).toEqual([]);
  expect(
    references.find(
      (reference) => reference.lessonId === "TA-C33-puri" && reference.word === "அது",
    ),
  ).toBeUndefined();
});
