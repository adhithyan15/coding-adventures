---
category: Repo policy / workflow reminders
---

# A keep-both conflict resolution must use git merge-file --diff3, or a closing brace both sides share is kept only once

main was merged into a Twig PR without a worktree. The conflicts were
resolved by keeping "ours then theirs" from plain `git merge-file` output.
Both sides had added a new test function, and both functions ended with the
same `    }` line.

Plain `git merge-file` merges zealously. It moved that shared closing brace
out of the conflict hunk and into the common context after it. Keeping both
sides of the hunk then left two function bodies and only one `}`, and the
build failed with "unclosed delimiter".

`git merge-file --diff3` (or `merge.conflictStyle=diff3`, but *not*
`zdiff3`, which is zealous again) turns zealous merging off. Each side then keeps its own lines, including
its own closing brace, so "keep both" is a straight concatenation. After any
mechanical keep-both resolution, build or at least parse the file before
committing.

A related trap: a parent branch was merged with main again, then merged into
its stacked child. This duplicated whole CHANGELOG sections, because the
same entries arrived by two paths. Merge main into each stacked branch
independently instead of re-merging a re-merged parent.
