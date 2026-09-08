import { expect, it } from "vitest";
import { defaultCurriculumRoot, loadChapterPolicy, loadTrackLessons } from "../../src/loader.js";
import { buildRootLedger } from "../../src/root-ledger.js";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
  languageWritingStages,
} from "./assert-language-corpus.js";
it("pins Arabic continuity", () => expectLanguageContinuity("arabic"));
it("pins Arabic modality", () => expectLanguageModality("arabic"));
it("pins Arabic lesson-content budgets", () =>
  expectLanguageLessonBudgets("arabic", {
    // HL-C286: 90 -> 102. Chapter 2's twelve lessons were schema v1, which
    // declares no atoms, so `book.ts` refused to generate the chapter and this
    // budget could not see them. Migrating them to v2 is what retired Arabic's
    // last hand-written chapter. RE-MEASURED against the tree, not derived: the
    // idiom, sense and culture-claim totals are unchanged at 2 / 3 / 14, because
    // the migration declared atoms and renamed headings without authoring new
    // vocabulary.
    //
    // 102 -> 123: the present-tense-and-joining tranche (chapters 37-41) adds
    // twenty-one lessons -- sixteen items and five reviews. RE-MEASURED against
    // the tree. Idioms, senses and culture claims stay at 2 / 3 / 14: a
    // conjunction is none of the three, and the one lesson that could have
    // claimed a culture note (يا, whose absence means a learner cannot address
    // anybody) states a GRAMMATICAL fact about the vocative particle.
    lessons: 123,
    idioms: 2,
    senses: 3,
    cultureClaims: 14,
    unitPrefix: "AR",
  }));
it("pins Arabic's complete pre-A1 writing ramp", () => {
  const arabic = languageWritingStages("arabic");
  expect(arabic.defects).toEqual([]);
  expect(arabic.levels[0]).toMatchObject({ level: "pre-A1", complete: true, missingStages: [] });
});
it("pins Arabic's root ledger", () => {
  const root = defaultCurriculumRoot();
  const ledger = buildRootLedger(
    loadTrackLessons("arabic", root),
    loadChapterPolicy(root).rootLedgerMinReuse ?? 3,
  );
  expect(ledger.summary).toEqual({
    // 104 -> 107. The chapters 37-41 tranche declares five roots: k-t-b and
    // f-h-m already existed and are now SPENT for the first time (payoff 0 -> 1
    // for both, which is the "1" bucket rising by two), and h-s-n, h-y-n and
    // k-r-r are new. Two of the three new ones are spent by a later lesson in
    // the same tranche and one is not, so neverSpent rises by exactly one.
    // Recorded rather than avoided: each of the three is the teaching point of
    // its own lesson -- ḥasanan IS the adjective ḥasan with an adverb ending,
    // ḥīnamā IS ḥīn plus mā, and karrir IS the doubled Form II of k-r-r -- so
    // dropping the declaration would hide an etymology the lesson is built on.
    // underspentPercent holds at 98, which is the number that would have moved
    // if this were padding.
    roots: 107,
    // HL-C285 retired the superseded AR-W01/W02/W03 writing ladder, which was
    // the only later spend of five Phoenician letter-origin roots (aleph, bet,
    // lamed, mem, shin) and of abjad-vowels. Their PROSE was re-homed into the
    // AR-W00 lessons that teach those same letters, so the reader still meets
    // every origin story -- but the ledger counts DECLARING LESSONS, not prose,
    // so a root introduced and re-used only inside one lesson reads as unspent.
    // This is a real, quantified cost of removing the duplicate ladder, recorded
    // rather than papered over by padding `roots:` onto downstream lessons.
    underspent: 105,
    neverSpent: 94,
    payoffDistribution: { "0": 94, "1": 10, "2": 1, "3": 1, "5": 1 },
    underspentPercent: 98,
  });
});
