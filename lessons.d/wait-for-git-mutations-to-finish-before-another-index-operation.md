---
category: Repo policy / workflow reminders
---

# Wait for Git mutations to finish before another index operation

A yielded fetch-and-merge command was still running when a spec commit started.
The operations raced on the worktree index; the first merge failed to write its
index. Preserve the committed spec, wait for the original process to exit, stage
the preserved file again and rerun the merge serially. A returned session ID is
not command completion. Always resolve the session before another Git mutation.
