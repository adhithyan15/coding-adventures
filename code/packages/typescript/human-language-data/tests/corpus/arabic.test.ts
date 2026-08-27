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
    lessons: 86,
    idioms: 2,
    senses: 3,
    cultureClaims: 11,
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
    underspent: 101,
    neverSpent: 88,
    payoffDistribution: { "0": 88, "1": 10, "2": 3, "3": 2, "6": 1 },
    underspentPercent: 97,
  });
});
