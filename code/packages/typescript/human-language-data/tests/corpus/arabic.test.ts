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
    lessons: 102,
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
    roots: 104,
    // HL-C285 retired the superseded AR-W01/W02/W03 writing ladder, which was
    // the only later spend of five Phoenician letter-origin roots (aleph, bet,
    // lamed, mem, shin) and of abjad-vowels. Their PROSE was re-homed into the
    // AR-W00 lessons that teach those same letters, so the reader still meets
    // every origin story -- but the ledger counts DECLARING LESSONS, not prose,
    // so a root introduced and re-used only inside one lesson reads as unspent.
    // This is a real, quantified cost of removing the duplicate ladder, recorded
    // rather than papered over by padding `roots:` onto downstream lessons.
    underspent: 102,
    neverSpent: 93,
    payoffDistribution: { "0": 93, "1": 8, "2": 1, "3": 1, "5": 1 },
    underspentPercent: 98,
  });
});
