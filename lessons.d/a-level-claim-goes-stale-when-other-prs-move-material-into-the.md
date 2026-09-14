# A level claim goes stale when OTHER PRs move material INTO the level

**What happened (#13061).** A branch drove Spanish's A1 reinforcement residue to
zero and asserted `attained: A1`. It then sat unmerged for a few hours. In that
window #13132 and #13144 moved DELE A1 verbs **down from A2 into A1**. Those
verbs' atoms carried their own reinforcement debt, and it landed at A1 because
that is where the verbs now lived. Rebasing turned the branch's headline
assertion false: ten atoms at or below A1, revisited fewer than twice.

The reflex is to think a level claim is threatened by PRs that ADD lessons to
that level. It is equally threatened by PRs that MOVE existing material into it,
and those are easier to miss because the corpus gained no lessons at all.

**Rule:** after rebasing any branch that asserts a level has been attained,
re-run the gate before trusting the branch's own numbers. `attained` is a
statement about a whole corpus, and a merge is a corpus change.
