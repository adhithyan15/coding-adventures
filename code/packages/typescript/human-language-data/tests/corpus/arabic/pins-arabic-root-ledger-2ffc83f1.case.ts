import { expect, it } from "vitest";
import { defaultCurriculumRoot, loadChapterPolicy, loadTrackLessons } from "../../../src/loader.js";
import { buildRootLedger } from "../../../src/root-ledger.js";

it("pins Arabic's root ledger", () => {
  const root = defaultCurriculumRoot();
  const ledger = buildRootLedger(
    loadTrackLessons("arabic", root),
    loadChapterPolicy(root).rootLedgerMinReuse ?? 3,
  );
  expect(ledger.summary).toEqual({
    // Chapters 37-41 brought the root set to 107. Retiring the superseded
    // AR-W01/W02/W03 ladder preserved its origin prose but removed several
    // downstream declaration spends, so that honest cost remains pinned here.
    //
    // The etymology pass (HL24 section 9) gave every word lesson the root or
    // donor its "taken apart" block names, checked against Lane's Lexicon:
    // 107 -> 943 roots. The new payoffs are reuses of one Arabic root as
    // Lane files it (maktab and kitab on k-t-b, mustaqbal and qabla on q-b-l);
    // loans that only look like a root (jazar 'carrot', khiyar 'cucumber')
    // keep slugs of their own. Most roots are still spent once or not at
    // all, which is the honest state of a vocabulary list.
    roots: 943,
    underspent: 915,
    neverSpent: 697,
    payoffDistribution: { "0": 697, "1": 165, "2": 53, "3": 19, "4": 7, "5": 1, "6": 1 },
    underspentPercent: 97,
  });
});
