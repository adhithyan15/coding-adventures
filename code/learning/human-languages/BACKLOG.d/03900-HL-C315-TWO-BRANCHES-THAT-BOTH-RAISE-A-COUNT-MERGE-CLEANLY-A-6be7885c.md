## HL-C315 — two branches that both RAISE a count merge cleanly at a value one short of the truth

Observed, not theorised. It happened twice in one merge.

### What happened

`plan-cli.test.ts` pinned two literals counting the exam inventories on disk:

    expect(out).toMatch(/0 complete and 10 partial of 138/);
    expect(out).toMatch(/0 complete and  9 partial of 138/);   // minus the one this test corrupts

Two branches were open at the same time. One added `exam-inventory-french-a2.json`
and raised both literals by one. The other added `exam-inventory-malayalam-a1.json`
and raised both literals by one. **Each was correct when it was written.**

Git merged the assertion lines **without a conflict**, because both sides had
written the identical new text — `11` and `10`. The conflict markers landed only
on the explanatory COMMENTS above them, which differed. Resolving those comments
looks like the whole job, and the assertion below them reads as already-merged
and settled.

The corpus now holds **twelve** inventories. The merged test asserted **eleven**
and passed.

### Why this is worse than the failure it looks like

A stale pin that DISAGREES produces a conflict, or a red test, and someone fixes
it. A stale pin that AGREES produces neither. The suite is green, the reviewer
sees a clean merge, and the assertion is now quietly measuring a corpus that is
one file smaller than the one on disk — and it will stay wrong until somebody's
change happens to move it again.

Nothing detects it. Not `--check`, not the merge, not review. HL-C310 removed the
sibling `uncovered point(s)` literal from this same file for a milder version of
this problem — churn — and left these two standing, because churn is annoying and
this failure is silent.

### The rule

**A count of files on disk must never be a literal in a test.** Derive it:

    const written = new Set(
      readdirSync(join(defaultCurriculumRoot(), "core"))
        .filter((file) => /^exam-inventory-.*\.json$/.test(file))
        .map((file) => `${doc.language}/${doc.level}`),
    );
    expect(out).toMatch(new RegExp(`0 complete and ${written.size} partial of 138`));

The derivation reads a source the plan engine does not produce, so it is a real
cross-check rather than `f(x) == x`. It asserts the two things the test actually
owns — **nothing is complete**, and **every written inventory is counted exactly
once** — and both survive any number of authors landing any number of inventories
in parallel.

Falsified before shipping, per the standing rule that a passing assertion proves
nothing until you have watched it fail: perturbed to `written.size + 1` and
confirmed it fails against real report output reading 12.

### Where else to look

Any pin whose value is "how many of X exist". The denominator `138` in the same
line is NOT one of these — it is tracks times certifiable levels, and it moves
only when a track is added, which is a deliberate act that should break a test.
