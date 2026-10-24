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
    //
    // The independent review then moved two lessons off native-root slugs:
    // miqlama joins qalam on greek-kalamos (one more payoff), and tawabil
    // takes aramaic-tabla, so 943 -> 942 roots.
    //
    // The etymology-hygiene pass unified two spellings of one root: the six
    // lessons on `semitic-s-l-m` (AR-C04's salāma and the AR-W00/W12 greeting
    // writing lessons) moved onto the bare `s-l-m` that AR-C01-salam already
    // introduced. 942 -> 941 roots; the root's payoffs go 2 + 5 -> 8 in one
    // entry, which is the reuse the split had been hiding.
    //
    // The B1 core (chapters 283-292) adds three new roots, ḥ-k-y (ḥakā),
    // ʾ-w-b (iyāb) and ʿ-m-d (iʿtamada): 941 -> 944. Its other word, phrase and
    // grammar lessons reuse roots the track already had (k-w-n, r-a-y, s-a-l
    // twice, w-ṣ-l, dh-h-b, ṭ-y-r, j-r-b, w-j-b, s-b-b), which is where the new
    // payoffs come from. RE-MEASURED against the tree, not derived.
    //
    // The B1 vocabulary (chapters 293-552, 1,300 headwords) adds 448 roots and
    // donor slugs, 944 -> 1392. It reuses the track's existing slug wherever a
    // root already had one (jamaa-gather, zahara-appear, ithnayn, ayn-f-w,
    // sawad-black, h-w-l ...), so B1 words on old roots turn once-spent roots
    // into payoffs: never-spent rises only 694 -> 804, while the share of
    // underspent roots falls 97% -> 89%. RE-MEASURED against the tree.
    roots: 1392,
    underspent: 1240,
    neverSpent: 804,
    payoffDistribution: {
      "0": 804,
      "1": 299,
      "2": 137,
      "3": 79,
      "4": 36,
      "5": 18,
      "6": 10,
      "7": 4,
      "8": 2,
      "9": 3,
    },
    underspentPercent: 89,
  });
});
