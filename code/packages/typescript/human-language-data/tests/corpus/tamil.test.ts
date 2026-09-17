import { expect, it } from "vitest";
import { measureContinuity } from "../../src/continuity.js";
import {
  defaultCurriculumRoot,
  loadChapterPolicy,
  loadEverything,
  loadExamInventory,
  loadTrackLessons,
} from "../../src/loader.js";
import {
  formatExamCoverage,
  measureExamCoverage,
  trackIntroducedAtoms,
} from "../../src/exam-inventory.js";
import { measureRamp, readingOrder } from "../../src/ramp.js";
import { measureScriptClosure } from "../../src/script-closure.js";
import { expectLanguageContinuity, expectLanguageModality } from "./assert-language-corpus.js";
it("pins Tamil continuity", () => expectLanguageContinuity("tamil"));
it("pins Tamil modality", () => expectLanguageModality("tamil"));
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

it("keeps Tamil Chapter 7 meaning-first and below the three-glyph step budget", () => {
  const root = defaultCurriculumRoot();
  const lessons = loadTrackLessons("tamil", root).sort(readingOrder);
  const chapter = lessons.filter((lesson) =>
    /^TA-(?:C07-numbers|W07-(?:digits|number-words|numbers))/.test(lesson.realization.lessonId),
  );
  expect(chapter.map((lesson) => lesson.realization.lessonId)).toEqual([
    "TA-C07-numbers-1-5",
    "TA-W07-digits-1-3",
    "TA-W07-digits-4-5",
    "TA-W07-number-words-1-5",
    "TA-W07-numbers-1-5-guided-copy",
    "TA-W07-numbers-1-5-delayed-copy",
    "TA-W07-numbers-1-5-dictation",
    "TA-C07-numbers-1-5-family",
    "TA-C07-numbers-6-10",
    "TA-W07-digits-6-8",
    "TA-W07-digits-9-10",
    "TA-W07-number-words-6-10",
    "TA-W07-numbers-6-10-guided-copy",
    "TA-W07-numbers-6-10-delayed-copy",
    "TA-W07-numbers-6-10-dictation",
    "TA-C07-numbers-6-10-family",
    "TA-C07-numbers-practice",
  ]);

  const spoken = chapter.filter((lesson) =>
    ["TA-C07-numbers-1-5", "TA-C07-numbers-6-10"].includes(lesson.realization.lessonId),
  );
  expect(spoken.every((lesson) => !lesson.body.match(/\p{Script=Tamil}/u))).toBe(true);
  expect(spoken.every((lesson) => lesson.frontmatter.skills?.join(",") === "listening,speaking")).toBe(true);

  const script = measureRamp(lessons, loadChapterPolicy(root)).script;
  expect(script.lessons.filter((lesson) => lesson.chapter === 7)).toEqual([]);
  expect(new Set(chapter.flatMap((lesson) =>
    [...lesson.body.matchAll(/hl-writing-stage:\s*([a-z-]+)/g)].map((match) => match[1]),
  ))).toEqual(new Set([
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
  ]));
});

it("teaches short o by meaning, then retrieves it before Chapter 8 uses it", () => {
  const lessons = loadTrackLessons("tamil").sort(readingOrder);
  const chapter = lessons.filter((lesson) =>
    /^TA-(?:C08-(?:tayavuseytu|sollungal|please-register)|W08-(?:short-o|sollungal))/.test(
      lesson.realization.lessonId,
    ),
  );
  expect(chapter.map((lesson) => lesson.realization.lessonId)).toEqual([
    "TA-C08-tayavuseytu",
    "TA-C08-sollungal",
    "TA-W08-short-o-observe",
    "TA-W08-sollungal-guided-copy",
    "TA-W08-sollungal-delayed-copy",
    "TA-W08-sollungal-dictation",
    "TA-C08-please-register",
  ]);

  const spoken = chapter.find((lesson) => lesson.realization.lessonId === "TA-C08-sollungal");
  expect(spoken?.body.match(/\p{Script=Tamil}/u)).toBeNull();
  expect(spoken?.frontmatter.skills?.join(",")).toBe("listening,speaking");
  expect(new Set(chapter.flatMap((lesson) =>
    [...lesson.body.matchAll(/hl-writing-stage:\s*([a-z-]+)/g)].map((match) => match[1]),
  ))).toEqual(new Set([
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
  ]));

  const closure = measureScriptClosure(lessons);
  const track = closure.tracks.find(
    (candidate) => candidate.language === "tamil",
  );
  expect(track?.neverTaughtGlyphs).toBe(0);
  // Zero, and pinned AS zero rather than re-pinned to a smaller count. The 21
  // that stood here were all one defect wearing twenty-one faces: chapters 1-6
  // printed Tamil words in running prose and recap tables beside their own
  // romanization, which the reader was never asked to decode and the script
  // strand does not reach until much later. Those chapters are the sound-first
  // opening, so the script came out of their bodies and stayed only in each
  // lesson's own romanized headword, where the exposure rule already covers it.
  // The last two -- ch9 and ch32 -- needed the letter உ, which was the one Tamil
  // letter with no lesson at all; TA-S125-letter-u now teaches it before either.
  // An exact zero, not a ceiling: a single new violation is a lesson asking the
  // reader to decode something nobody taught, and there is no longer a backlog
  // for it to hide inside.
  expect(track?.violations).toBe(0);
  // HL-C194: every Tamil headword now declares how to say it, so no headword is
  // load-bearing script. Pinned at zero rather than at a count, because this is
  // the one number in the closure report that an author can only make worse by
  // shipping a lesson whose headword nobody can pronounce.
  expect(track?.headwordsWithoutRomanization).toBe(0);
  expect(closure.violations.filter((violation) =>
    violation.language === "tamil" && (
      violation.glyphs.includes("ொ") ||
      ["TA-C08-please-register", "TA-C20-pathinondru-irupathu"].includes(violation.lessonId)
    )
  )).toEqual([]);
});

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
