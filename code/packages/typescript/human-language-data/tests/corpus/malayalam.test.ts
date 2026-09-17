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
import {
  expectLanguageContinuity,
  expectLanguageModality,
  languageWritingStages,
} from "./assert-language-corpus.js";
it("pins Malayalam continuity", () => expectLanguageContinuity("malayalam"));
it("pins Malayalam modality", () => expectLanguageModality("malayalam"));
it("keeps Malayalam's opening free of genuine future farewells and pronouns", () => {
  const references = measureContinuity(
    loadTrackLessons("malayalam", defaultCurriculumRoot()),
  ).forwardReferences;
  expect(references.length).toBeLessThanOrEqual(12);
  expect(references.filter((reference) => /-C0[12]-/.test(reference.lessonId))).toEqual([]);
});
it("keeps the santosham payoff inside the three-glyph lesson budget", () => {
  const root = defaultCurriculumRoot();
  const report = measureRamp(loadTrackLessons("malayalam", root), loadChapterPolicy(root)).script;
  expect(report.lessons.find((lesson) => lesson.lessonId === "ML-C02-santosham")).toBeUndefined();
});

it("keeps Malayalam Chapter 7 meaning-first and below the three-glyph step budget", () => {
  const root = defaultCurriculumRoot();
  const lessons = loadTrackLessons("malayalam", root).sort(readingOrder);
  const chapter = lessons.filter((lesson) => /^ML-[CW]07-/.test(lesson.realization.lessonId));
  expect(chapter.map((lesson) => lesson.realization.lessonId)).toEqual([
    "ML-C07-numbers-1-5",
    "ML-W07-digits-1-3",
    "ML-W07-digits-4-5",
    "ML-W07-number-words-1-5",
    "ML-W07-numbers-1-5-guided-copy",
    "ML-W07-numbers-1-5-delayed-copy",
    "ML-W07-numbers-1-5-dictation",
    "ML-C07-numbers-6-10",
    "ML-W07-digits-6-8",
    "ML-W07-digits-9-10",
    "ML-W07-number-words-6-10",
    "ML-W07-numbers-6-10-guided-copy",
    "ML-W07-numbers-6-10-delayed-copy",
    "ML-W07-numbers-6-10-dictation",
    "ML-C07-numbers-practice",
  ]);

  const spoken = chapter.filter((lesson) => lesson.realization.lessonId.startsWith("ML-C07-numbers-") && lesson.realization.lessonId !== "ML-C07-numbers-practice");
  expect(spoken).toHaveLength(2);
  expect(spoken.every((lesson) => !lesson.body.match(/\p{Script=Malayalam}/u))).toBe(true);
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

it("keeps Malayalam's first greeting meaning-first and its script runway learner-visible", () => {
  const opening = loadTrackLessons("malayalam").sort(readingOrder).slice(0, 9);
  expect(opening.map((lesson) => lesson.realization.lessonId)).toEqual([
    "ML-C01-namaskaram",
    "ML-W01-na-ma-trace",
    "ML-W01-na-ma-guided-copy",
    "ML-W01-na-ma-delayed-copy",
    "ML-W01-na-ma-dictation",
    "ML-W01-sa-chandrakkala-ka",
    "ML-W01-aa-ra-anusvaram",
    "ML-W01-namaskaram-read",
    "ML-W01-namaskaram-dictation",
  ]);
  expect(opening.every((lesson) => lesson.frontmatter.chapter === "1")).toBe(true);
  expect(opening[0]?.frontmatter.skills).toEqual(["listening", "speaking"]);
  expect(opening[0]?.body).not.toMatch(/\p{Script=Malayalam}/u);
});

it("gives Malayalam a complete pre-A1 writing runway", () => {
  const malayalam = languageWritingStages("malayalam");
  expect(malayalam.defects).toEqual([]);
  expect(malayalam.levels[0]).toMatchObject({
    level: "pre-A1",
    complete: true,
    missingStages: [],
  });
  expect(new Set(malayalam.validEvidence.map((entry) => entry.stage))).toEqual(new Set([
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
  ]));
});

// ---------------------------------------------------------------------------
// THE MALAYALAM A1 INVENTORY HAD NO ASSERTION IN THIS FILE -- the hole HL-C354
// found in Telugu and Hindi and told the next reader to look for in the other
// eighteen. Without these two tests the ordinal tranche could land its atoms,
// wire ML-A1-NUM-05's probe, and leave a coverage number that nothing in the
// track's own test file reads. Both halves were falsified before being kept: a
// fabricated atom id in the probe fails the first, and nulling ML-A1-NUM-05's
// probe fails the second.
// ---------------------------------------------------------------------------
it("probes only Malayalam atoms that EXIST, so a guessed id cannot under-report", () => {
  const { lessons } = loadEverything();
  const taught = trackIntroducedAtoms(lessons, "malayalam");
  const unknown: string[] = [];
  for (const point of loadExamInventory("malayalam", "A1").points) {
    for (const atom of point.probe ?? []) if (!taught.has(atom)) unknown.push(`${point.id}:${atom}`);
  }
  expect(unknown).toEqual([]);
}, 60_000);

it("pins Malayalam A1 coverage, and the ordinal point the tranche closed", () => {
  const { lessons } = loadEverything();
  const coverage = measureExamCoverage(loadExamInventory("malayalam", "A1"), lessons);
  expect(coverage.enumerated).toBe(243);
  // 163 -> 164: ML-A1-JOIN-02 (or). ONE POINT FOR FOUR LESSONS, AND THE RATIO
  // IS THE WRONG WAY TO READ THIS CHANGE. What chapter 70 actually does is give
  // Malayalam its 'and': the language coordinates with the clitic -um repeated
  // on EACH item, and NOTHING IN 307 LESSONS TAUGHT IT. Verified rather than
  // taken from the note -- -um appeared in exactly one lesson file, inside an
  // etymology note as a component of the word for evening, and the nine files
  // carrying the sequence u+m all had it INSIDE a word (veendum, kudumbam,
  // hrudayam, the month names). A learner owning 69 chapters of vocabulary could
  // not say 'water and rice'.
  // ML-A1-JOIN-01 STAYS OPEN ON PURPOSE. Its label is 'joining two nouns, AND
  // joining two clauses', and this chapter delivers the noun half only. The
  // everyday clause link in this corpus is the -i participle already covered by
  // ML-A1-JOIN-11, and claiming clause coordination off the noun lessons would
  // claim a range the corpus does not teach. Same call as TE-A1-L-10.
  // ML-A1-JOIN-04 (distributive) also stays open, but its blocker is gone: its
  // note said it depended on the missing coordinator, which now exists.
  // -o IS TAUGHT BESIDE -um BECAUSE THEY ARE ONE HABIT: both go on every item
  // with nothing in the gap, and only the vowel differs -- veLLavum ariyum for
  // both, chaayayoo kaappiyoo for one of them.
  // MALAYALAM HAS NO UNTRANSFERABLE POINTS, unlike Kannada's four, so its
  // ceiling is the full 243.
  // 164 -> 166: ML-A1-PRON-03 (third person) and ML-A1-PRON-04 (first and second
  // person plural). VERIFIED WORSE THAN THE NOTES SAID: avan, aval, avar and
  // njangal appeared in ZERO lesson files -- not as headwords, not anywhere in a
  // body. A learner with seventy chapters of vocabulary could say I and you and
  // could not say he, she, they or we.
  // naam LOOKED taught and was not: it appeared as a headword in ML-C67-first,
  // ML-C67-third and ML-C68-eleventh, where it is a SUBSTRING of the ordinals
  // onnaam, moonnaam and pathinonnaam. Every ordinal ending -nnaam is a false
  // positive for naam -- the same trap the Hindi campaign recorded when every
  // ordinal matched its own cardinal. Check the token, not the substring.
  // THE GAP SAT INSIDE A SYSTEM ALREADY TAUGHT. ML-C41-that teaches the i-/a-
  // pointing pair and says in as many words that a- means far, and the corpus
  // only ever used it on THINGS. avan, aval and avar carry the same a-, so the
  // column for people was predicted and never filled -- the same shape as Tamil's
  // TA-A1-PRON-03, closed in the chapter before this one. avar additionally
  // reuses ML-C02's own rule that a plural raises the register (nii -> ningal,
  // avan -> avar), so two of the three new words run on machinery already held.
  // 166 -> 167: ML-A1-JOIN-06, the quotative ennu. THE NOTE WAS VERIFIED BY
  // TOKEN RATHER THAN SUBSTRING, and that mattered: the string `enna` appears
  // nine times in the corpus, and every one is inside ennaal, the word for
  // "but" that ML-C64 teaches. The quotative -- with the virama -- was nowhere.
  // ONE MARKER BUYS BACK THE WHOLE CORPUS: ennu leaves the quoted sentence
  // untouched, so every sentence the reader can build becomes something they
  // can report, think, claim to know or ask. Two atoms, four verb frames, all
  // four verbs already taught.
  // THE SAYING-VERB WAS A SECOND FINDING AND IS FIXED IN CHAPTER 50, NOT 72:
  // parayuka was never a headword anywhere, while ML-C50-farewell built
  // `vita parayuka` and called it "the speaking-verb" for want of a name.
  // Teaching it at 72 made those uses forward references 104 and 106 lessons
  // early; the lesson moved to sequence 1405, immediately before the first use,
  // which returned forwardReferences to its baseline of 12 and paid a debt that
  // predated this work.
  // 167 -> 169: ML-A1-JOIN-05 (because) and ML-A1-Q-07 (why), which are ONE
  // PIECE OF WORK and whose notes said so -- cause could be handled in NEITHER
  // direction, so closing one without the other leaves a learner able to ask a
  // question nobody can answer. Both notes were verified: entukondu, kaaranam
  // and entukondennaal each returned ZERO files across the whole corpus.
  // MOST OF THE CHAPTER IS BUILT FROM WHAT THE READER HAD. Malayalam has no
  // separate word for why: entukondu is chapter two's entu plus kondu, "by
  // means of", so the question asks "by what". The written because then carries
  // that whole question word visibly at its front.
  // THE TAIL OF entukondennaal IS DELIBERATELY NOT TAKEN APART: its -ennaal can
  // be read as the but-word ML-C64 teaches or as a conditional of the saying
  // verb, grammars differ, and the lesson says so rather than picking one.
  // A DRAFT CLAIMED ALL FOUR QUESTION WORDS SHARE THE FRONT LETTER e AND WAS
  // WRONG: aaru (who) opens on aa. Three of the four carry the asking letter
  // ML-C41-deixis-system named, and who is the exception.
  expect(coverage.covered).toBe(169);
  expect(coverage.unmapped).toBe(74);
  expect(coverage.partial).toBe(0);
  // ML-A1-NUM-05 was one of the thirteen ordinal points HL-C354 left open.
  // Malayalam's -aam has no exceptions at all, so all eleven ordinals follow
  // from one ending on cardinals chapter 7 already taught, and the tranche also
  // closes the "ordering notion" the old note named as missing by teaching
  // aadyam against the pinne the track already had.
  expect(coverage.byCategory["Sankhya (numerals and quantity)"]!).toEqual({
    enumerated: 9,
    covered: 7,
  });
  expect(formatExamCoverage(coverage)).toContain(
    "malayalam A1 (partial inventory): 169/243 points covered (70%)",
  );
}, 60_000);
