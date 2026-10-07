---
category: Repo policy / workflow reminders
---

# A successful remote PR merge can still fail local checkout cleanup when main is held by another worktree

CV03 `gh pr merge --squash --match-head-commit <reviewed-head> --delete-branch` completed the remote merge but then failed local cleanup: `main` was already held by another worktree. Treat the remote PR API, expected reviewed head, fetched-main reachability and reviewed file blobs/modes as authority; do not retry the merge from the CLI exit status alone. Omit local checkout/deletion side effects when merging a managed worktree, and archive it through the managed lifecycle after preserving ignored validation artifacts. The auto-deleted source branch can still be verified with `refs/pull/<number>/head`; never recreate it to repair local cleanup.
