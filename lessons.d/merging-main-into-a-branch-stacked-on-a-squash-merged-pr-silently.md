---
category: Repo policy / workflow reminders
---

# Merging main into a branch stacked on a squash-merged PR silently resurrects code the branch removed

PR #18365 (Trap.ConvRange) was stacked on #18299 (JIT conversions). #18299 added a
`real_to_int:` error pass-through to jit-core, and #18365 removed it again.
When #18299 was squash-merged, #18365 became conflicted. A plain
`git merge origin/main` then *put the removed pass-through back*, and git
reported no conflict for it. The merge base was the main commit from before
#18299. Against that base the squash commit "adds" the lines, and the stacked
branch looks as if it never touched them (it added them, then deleted them), so
git takes main's side.

Fix: do not let git pick the old merge base. Merge in three steps:

1. `git merge <squash>^1`, which is main up to just before the squash.
2. `git merge -s ours <squash>`, which records that the branch already holds
   the squashed PR's content, in its newer form.
3. `git merge origin/main`, for everything after the squash.

Then check that `git diff --stat origin/main..HEAD` equals the branch's own
reviewed diff (here 34 files, +819/−158). A plain merge that silently
re-adds code shows up there as extra lines.

The PR7 stack (#18377 → #18384 → #18391 → #18400 → PR7e) hits this at
every squash, so use the three-step merge for each one.
