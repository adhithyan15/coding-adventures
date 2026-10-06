import { readFileSync } from "node:fs";
import { join } from "node:path";
import { expect, it } from "vitest";
import { compileLessonActivities } from "../../../src/activity.js";
import { measureContinuity } from "../../../src/continuity.js";
import {
  DOC_SHARD_PLANS,
  defaultRepoRoot,
  unshardDocContents,
} from "../../../src/doc-shard-cli.js";
import { defaultCurriculumRoot, loadTrackLessons } from "../../../src/loader.js";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
  languageWritingStages,
} from "../assert-language-corpus.js";

it("closes Chapter 3's oral R1-R4 windows without inventing script credit", () => {
  const ordered = loadTrackLessons("punjabi").sort(
    (left, right) => Number(left.frontmatter.sequence) - Number(right.frontmatter.sequence),
  );
  const checkpointIds = [
    "PA-R03-wellbeing-r1",
    "PA-R04-wellbeing-r2",
    "PA-R07-wellbeing-r3",
    "PA-R18-wellbeing-r4",
  ];
  const checkpoints = checkpointIds.map((id) =>
    ordered.find((lesson) => lesson.realization.lessonId === id)!,
  );
  // Punjabi A1: chapter 6's two over-budget lessons split into three -more
  // continuations ahead of this point, so these positions move by 3.
  expect(checkpoints.map((lesson) => ordered.indexOf(lesson))).toEqual([27, 36, 63, 132]); // +2: chapter 7's two continuations (hona, khana)
  expect(checkpoints.map((lesson) => lesson.frontmatter["introduces.knowledge"])).toEqual([
    [],
    [],
    [],
    [],
  ]);
  expect(checkpoints.every((lesson) => lesson.frontmatter.skills?.includes("listening"))).toBe(true);
  expect(checkpoints.every((lesson) => lesson.frontmatter.skills?.includes("speaking"))).toBe(true);
  expect(checkpoints.every((lesson) => !lesson.frontmatter.skills?.includes("reading"))).toBe(true);
  expect(checkpoints.every((lesson) => !lesson.frontmatter.skills?.includes("writing"))).toBe(true);
  expect(checkpoints.flatMap((lesson) => compileLessonActivities(lesson.blocks))).toHaveLength(11);
  // Chapter 4 used to be hand-written .tex with the R2 checkpoint spliced in behind a
  // `canonical-insertion` comment -- a chapter no lesson-level gate could see. It is now
  // GENERATED from its lessons, so the checkpoint is a canonical lesson of the chapter
  // rather than a comment promising one. That is the stronger claim, and it is what this
  // assertion now makes: the header must name the lesson, and its prose must be present.
  const generatedChapter4 = readFileSync(
    join(defaultCurriculumRoot(), "punjabi", "book", "chapters", "ch04-farewells.tex"),
    "utf8",
  );
  expect(generatedChapter4.startsWith("% GENERATED FILE.")).toBe(true);
  expect(generatedChapter4).not.toContain("canonical-insertion:");
  expect(generatedChapter4).toContain("% canonical-lessons: ");
  expect(generatedChapter4).toContain("PA-R04-wellbeing-r2");
  expect(generatedChapter4).toContain("label{lesson:PA-R04-wellbeing-r2}");
  expect(generatedChapter4).toContain("Romanization records speech only");

  const chapter3Atoms = new Set([
    "PA-GRAMMAR-TUSI-HO-03",
    "PA-ETYMON-QUESTION-K-03",
    "PA-ETYMON-FIRST-PERSON-M-03",
    "PA-ETYMON-NEGATIVE-NE-03",
    "PA-GRAMMAR-MAIN-HAAN-03",
    "PA-LEX-MAIN-03",
    "PA-LEX-THIK-03",
    "PA-PHRASE-HOW-ARE-YOU-03",
    "PA-PHRASE-I-AM-FINE-03",
    "PA-PHRASE-NO-PROBLEM-03",
    "PA-LEX-KIVEIN-03",
  ]);
  const report = measureContinuity(ordered);
  expect(report.reinforcement.filter((defect) => chapter3Atoms.has(defect.atom))).toEqual([]);
  // {45, 94, 141, 74} -> {54, 112, 157, 71}. Chapters 31-36 introduce 45 new atoms, and
  // the ones taught in the closing sessions have no room left after them for an R1 or R2
  // return, which is where the R1/R2/R3 growth comes from. R4 FALLS, from 74 to 71: the
  // new lessons retrieve mainu, madad and the wellbeing answers at long distance, which
  // is the window the earlier tranches were least able to reach.
  // #13068 inserted an 18-lesson recognition runway into Chapters 2-13 and gave
  // it a review layer: each runway lesson rehearses its three-lesson R1
  // neighbourhood, each early content lesson declares the letters its page
  // shows, and each later FORMATION lesson declares the recognition atom for the
  // same glyph. R1, R2 and R3 all fall BELOW their pre-runway values as a
  // result. R4 rises: 40 recognition atoms entered the corpus and 18 of them
  // still have no lesson 80-250 sessions later that puts their glyph back on the
  // page. That residue is named in BACKLOG.d as the next tranche's work.
  // Chapters 4 and 5 stopped being hand-written .tex and became generated from their
  // lessons. Migrating those ten lessons to schema v2 (and splitting two of them)
  // declared 25 knowledge atoms the corpus had been teaching in prose and counting
  // nowhere. Every window they do not close is now VISIBLE, which is why these totals
  // rise rather than fall: the numbers moved because the measurement reaches further,
  // not because reinforcement got worse. The serviced-debt assertions above still hold
  // exactly. The residue -- Chapter 4 and 5 atoms with no later lesson putting them
  // back in front of the reader -- is named in BACKLOG.d as the next tranche's work.
  // Chapter 45, the reading rung, moves four of these and the direction is not
  // uniform: R4 FALLS by five and R2/R3 rise by four. The fall is the reading
  // lessons pulling form labels, joiners and pronouns taught long ago back in
  // front of the reader at a long interval, which is exactly what an R4 window
  // is for. The rise is the three new reading skills, introduced in the last
  // chapter with nothing after them yet to close their own R2 and R3. That
  // residue is the instruction for chapter 46, not a reason to hold the rung.
  // {54, 130, 216, 106} -> {55, 132, 217, 107}. The timed A1 writing paper is
  // the LAST lesson in the track, and a terminal lesson cannot be retrieved: it
  // practises five form atoms, opening a window for each that nothing after it
  // can close. +1/+2/+1/+1 is the whole cost, it is inherent to any lesson that
  // sits at the end rather than a defect in this one, and the next lesson added
  // after it will pay part of it back.
  // {55, 132, 217, 107} -> {55, 135, 215, 90}. HL-C437's chapter 48: six `review`
  // lessons closing the track's pre-A1 reinforcement debt, appended after the
  // timed writing paper the comment above calls terminal. Decomposed by diffing
  // the (atom, window) pairs against the corpus measured without them, because a
  // fall of seventeen in R4 says nothing on its own about whose window closed:
  //
  //   CLOSED  8 R3 + 22 R4 = 30, by the six lessons retrieving forty atoms from
  //           chapters 4-36 at the far end of the book. R4 is the window those
  //           atoms were least able to reach, and it is where most of the fall is.
  //   OPENED  3 R2 + 6 R3 + 5 R4 = 14, and NOT ONE of them belongs to a lesson
  //           this tranche added. They are late-chapter atoms -- the phone
  //           fields, the pronouns, the question words, connected reading --
  //           whose windows did not EXIST at 272 lessons and do at 278.
  //
  // That second half is the comment above running in reverse. A terminal lesson
  // cannot be retrieved, so its windows are uncountable rather than closed; the
  // moment something follows it they become countable and start out missed. The
  // prediction there -- "the next lesson added after it will pay part of it
  // back" -- is what the 22 closed R4 windows are.
  // {55, 135, 215, 90} -> {55, 359, 379, 260}. Chapters 49-98 add 250 words,
  // each practised by the next two lessons and by one of four reviews, so their
  // R1 windows close and the level gate's two-revisit criterion holds. What they
  // do not yet have is a lesson 5-250 sessions later putting them back in front
  // of the reader: the growth is R2-R4 windows that now EXIST and start out
  // missed, exactly as the terminal-lesson note above describes. The Chapter 3
  // atoms this test is about are unaffected (the reinforcement filter above).
  // {55, 359, 379, 260} -> {55, 360, 380, 259}. HL-C443's ਮੌਸਮ (mausam)
  // before the chapter-20 ੌ lesson: its own R2, R3 and R4 windows start out
  // missed (+1 each), and the one extra lesson carries ਪ and ਕ into their R4
  // for the first time (-2 R4).
  // {55, 360, 380, 259} -> {55, 560, 484, 531}. Punjabi A1: chapters 99-152
  // (270 words, twelve reviews) and three chapter-6 continuations. Decomposed
  // against the corpus without them:
  //   +485 on the new chapters' own atoms (201 R2, 90 R3, 194 R4): they sit at
  //        the end of the track, and no later lesson exists to revisit them.
  //    +99 on chapters 75-98 (3 R2, 18 R3, 78 R4): their windows used to fall
  //        past the end of a 533-lesson track and are now measurable.
  //     -8 PRE-EXISTING closed (4 R2, 4 R3) by the chapter-6 continuations.
  //      0 pre-existing windows lost: the splits pushed the top-line clue, the
  //        addak and the tippi past R1, and each is retrieved by name in the
  //        next lesson's warm-up.
  // {55, 560, 484, 531} -> {55, 559, 482, 531}: chapter 7's hona and khana each
  // split an atom-budget continuation off. Decomposed against the corpus without
  // them: -1 R2 and -1 R3 on PA-COMPARISON-PANJ-MATCH, -1 R3 on the top-line
  // clue, both closed because the insertions move a later retrieval into the
  // window; 0 lost, because the two panj atoms the hona insertion pushed past R1
  // are retrieved by name in PA-C07-hona-two-roots' warm-up.
  // Chapter 153 extends the tail by 11 short date-writing sessions. One R1
  // and three R2 windows newly become measurable; all 11 create R3/R4 windows
  // for earlier atoms in the measured report.
  // Chapter 154 adds seven short integration sessions after the date runway.
  // Their new tail makes one R2 and seven R3/R4 windows measurable. The
  // independent form also revisits date-format repair within its R2 window,
  // closing that one missed retrieval; the chapter-3 assertions still guard
  // the original debt.
  // Chapter 155 adds six short form sessions at the tail. The supported
  // and delayed six-field skills are genuinely revisited twice, closing two
  // R1 misses; one R2, four R3, and six R4 misses remain measurable debt.
  // Chapter 156 extends the measurable tail by 19 short, no-new-atom writing
  // sessions. Two genuine six-field form recalls close one R1 and two R2
  // windows; the remaining newly measurable debt is retained, not suppressed.
  // Chapter 157 adds five short timed-writing and repair sessions. A direct
  // before/after (atom, window) diff shows two R3 closures on the six-field
  // cue and repair atoms. Five Chapter 146 quality-word R4 windows become
  // measurable at the new tail; #16793 owns their later retrieval. The test
  // pins the measured debt rather than calling these unrelated words taught
  // by the timed form or message.
  expect(report.summary.missedByWindow).toEqual({ R1: 56, R2: 566, R3: 513, R4: 577 });
});
