import { expect, it } from "vitest";
import { defaultCurriculumRoot, loadChapterPolicy, loadTrackLessons } from "../../src/loader.js";
import { buildRootLedger } from "../../src/root-ledger.js";
import { expectLanguageContinuity, expectLanguageModality } from "./assert-language-corpus.js";
it("pins Arabic continuity", () => expectLanguageContinuity("arabic"));
it("pins Arabic modality", () => expectLanguageModality("arabic"));
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
