---
category: Repo policy / workflow reminders
---

# git reset --soft origin/main then commit reverts everything main changed since the branch point

Squashing a branch with

    git reset --soft origin/main && git commit

is a trap in a fast-moving repo. `--soft` moves HEAD and leaves the index
alone, so the new commit's TREE is your working tree while its PARENT is the
current `origin/main`. Every file main changed since you branched is reverted,
and every file main added is DELETED -- in a commit whose message describes
none of it.

In #15279 that produced a commit touching 102 files, 61 of them deletions:
Arabic book chapters, gentle-ramp snapshot metrics, two append-only
`CHANGELOG.d` shards. The intended change was four files.

Two things make it hard to see:

- **A later `git merge origin/main` does not fix it.** Git reads the missing
  files as a deliberate deletion on your side and keeps the deletion.
- **CI mostly does not catch it.** The append-only shard guard flagged 2 of
  the 61; nothing would have flagged the other 59.

## Do not diagnose it with `git diff main branch`

That diff counts files main ADDED after the branch point as deletions, so a
perfectly healthy branch looks catastrophic. It made a second PR in the same
session look like it had 92 deletions when its commit had none, and a routine
`git merge origin/main` resolved it to exactly its 4 intended files.

The honest check is the commit against ITS OWN parent:

    git show --name-status <sha> | grep -cE '^D\s'

Zero deletions there means the branch is fine. Deletions there are yours.

## What to do instead

- To squash: `git reset --soft $(git merge-base origin/main HEAD)` -- the
  merge base, never the moving branch tip.
- Then `git merge origin/main` as a separate step, so the update to main is a
  merge with a real base rather than a silent tree swap.
- Before pushing any squashed branch, assert the scope:
  `git show --name-status HEAD | grep -cE '^D\s'` and confirm the changed-file
  list is the one you intended.
