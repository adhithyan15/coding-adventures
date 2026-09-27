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
    roots: 107,
    underspent: 105,
    neverSpent: 94,
    payoffDistribution: { "0": 94, "1": 10, "2": 1, "3": 1, "5": 1 },
    underspentPercent: 98,
  });
});
