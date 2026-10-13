---
category: Repo policy / workflow reminders
---

# Run post-merge measurements against the recorded merge SHA, not the moving main tip

After PR #17435 merged, the cleanup worktree was initially created from the
then-current `origin/main`, which had already advanced beyond the PR merge.
That checkout was suitable for the next implementation but not for the promised
post-merge measurement: a moving main tip changes the history window and makes
the result impossible to attribute to the completed PR. Record the merge commit
from `gh pr view` and pass that full SHA explicitly to read-only audit tools.
Create later feature branches from the newest fetched main independently; do
not make the audit revision and the next implementation base stand in for one
another.
