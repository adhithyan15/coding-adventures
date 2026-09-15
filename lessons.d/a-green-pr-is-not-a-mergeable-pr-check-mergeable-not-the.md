# A green PR is not a mergeable PR — check `mergeable`, not the check marks

**Symptom:** fourteen of fifteen human-language PRs merged on their own. The
fifteenth, #13815 (the Hindi romanization tranche, the largest single metric move
of the batch — load-bearing headwords 71 -> 1), sat open for a day with
**auto-merge armed, every check green, and zero failures**. Nothing surfaced it.
The owner found it by looking at the PR list.

**Root cause:** GitHub's check marks answer "did CI pass on this head?" They say
nothing about whether the branch still applies to `main`. #13815's base moved 106
commits while it was open — including other Hindi work that touched the same
chapters — so its `mergeable` flipped to `CONFLICTING` / `mergeStateStatus:
DIRTY`. **A conflicted PR reports no failing checks at all**, because there is
nothing to fail: CI already ran, on the old head, and passed. `gh pr merge
--auto` is a conditional that silently no-ops in this state, so arming it reads
as completion while nothing is happening.

**Why a monitor did not catch it:** the watcher polled `mergeable` and treated
only `CONFLICTING` as a conflict. GitHub computes mergeability **lazily** — for a
window after every push (and after every base change) the field reads `UNKNOWN`,
not `CONFLICTING`. A poll that fires inside that window sees nothing wrong. On a
repo taking ~100 commits/day to `main`, that window reopens constantly.

**How to detect fast:**

```
gh pr list --state open --json number,mergeable,mergeStateStatus \
  --template '{{range .}}{{.number}} {{.mergeable}} {{.mergeStateStatus}}{{"\n"}}{{end}}'
```

`MERGEABLE`+`BLOCKED` is normal (checks pending). `CONFLICTING`+`DIRTY` is dead
in the water. `UNKNOWN` is **not** reassurance — it means GitHub has not computed
it yet, so re-read rather than move on.

**Fix:** merge `origin/main` into the branch, resolve, push again. Two rules that
matter more than the resolution itself:

* **Regenerate generated artifacts; never hand-merge them.** Every conflict in
  #13815 was in generated output — narration, book chapters, lesson modality,
  book/narration hashes, gentle-ramp snapshots. Taking either side of a textual
  conflict produces a file that matches neither input. Take one side to clear the
  conflict, then re-run every generator and let the output be the truth. Only
  hand-merge authored files (there, one CHANGELOG).
* **Re-verify after resolving.** A clean textual merge can still be wrong. Re-run
  the gates and re-measure the metric the branch existed to move — #13815's
  headline number was confirmed to survive at 1 before it was pushed.

**The habit this should install:** never end a work session on "N PRs are in
flight." End it on a freshly-read state table for every open PR. A watcher is not
evidence; arming auto-merge is not babysitting. See also *Auto-merge can
immediately merge a stacked PR into an unprotected feature base* above, and *A
stale-branch merge can silently REVERT already-merged work* — all three are the
same mistake: trusting a green signal that was never answering the question.

Discovered: 2026-09-01, human-languages Indic wave-1 batch (15 PRs).
