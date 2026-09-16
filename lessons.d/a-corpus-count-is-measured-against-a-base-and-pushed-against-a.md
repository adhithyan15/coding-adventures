---
category: Repo policy / workflow reminders
---

# A corpus count is measured against a base and pushed against a tip, and those are different numbers

A commit message asserted **"537 shards, 0 problems (unchanged)"** and, separately, that a defect
appeared in **"the only one of 537 shards"**. Both numbers were measured, by me, against the base
the branch started from. Between the commit and the push, `origin/main` moved and two unrelated
shards landed. At the tip the same command returned **539**.

Nothing about the file under review had changed. The *corpus* had.

**The window is between measuring and pushing, and it is wider than it looks.** The measurements
ran before the commit; the fetch ran after it, as a routine pre-push step. Had the fetch not
happened to run before `git push`, both numbers would have shipped describing a corpus that no
longer existed — and a reader re-running `lessons.py validate` on the merge commit would get 539
and have no way to tell whether the message was wrong, the corpus had grown, or they were counting
something else.

**What the repair was, and what it was not.** It was not re-explaining the number, and it was not
deleting it. It was naming the side each count came from:

    -> 539 shards, 0 problems, measured at this branch's TIP after merging
       origin/main 073c957697. It was 537 at the base this work started
       from (e2b063126b); two shards arrived meanwhile with #15390, neither of
       them this one.

A count that names its base is reproducible by anyone holding that base. A bare count is only
reproducible by whoever happened to run it that hour.

**A universal claim over a corpus has to be re-derived, not carried.** The second number sat inside
*"the only one of N shards that…"*. That is a claim about every file in the corpus, so a corpus that
grew by two is a corpus with two unchecked members. Re-running the census at the tip was the only
thing that could settle it — it held, **1 of 540** at `073c957697`, but it held as a measurement
rather than as an assumption that survived because nobody looked. Re-run once more on this branch
(base `34e305c158`, this shard present) it reads **1 of 541**. Same claim, same predicate, moved
denominator — and the only reason those two readings can be told apart is that each names its
commit.

**State the denominator's population, not just its size.** At `073c957697` the census walked **540**
`.md` files while `validate` reported **539** shards. The gap is `_meta.md`, which is not a shard. Two
defensible numbers describe the same directory, so *"1 of 539"* and *"1 of 540"* are both true of
different populations and neither is self-explanatory. The shipped wording names which: *1 of the
540 `.md` files under `lessons.d/` (539 shards plus `_meta.md`)*.

**The operational form** is to re-run corpus-wide measurements after the last fetch that precedes
the push, not before the commit that contains them — and to write every such number with its base
attached, so that a later reader can tell a stale count from a wrong one. A count with no side
named cannot be checked; it can only be believed.

Related: [[a-level-claim-goes-stale-when-other-prs-move-material-into-the]] — the same corpus-wide
assertion falsified by an unrelated merge, but by a different mechanism: there the corpus gained no
lessons at all and material *moved* into the level, and the remedy is re-running the gate after a
rebase. Here the corpus genuinely grew, and the remedy is naming which side the number came from.
