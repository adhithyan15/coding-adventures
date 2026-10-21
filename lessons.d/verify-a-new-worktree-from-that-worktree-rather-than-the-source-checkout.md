---
category: Repo policy / workflow reminders
---

# Verify a new worktree from that worktree rather than the source checkout

`git worktree add` changes repository state but does not change the shell's
working directory. A trailing `git rev-parse HEAD` in the same command therefore
reports the source checkout's HEAD, which can make a correctly based worktree
look stale. After creation, run branch, HEAD, and status verification with the
new worktree as the command working directory; do not interpret source-checkout
output as evidence about the new tree.
