---
category: Repo policy / workflow reminders
---

# Wait for Git mutations to finish before another index operation

A yielded fetch-and-merge command was still running when a spec commit started.
The operations raced on the worktree index; the first merge failed to write its
index. Preserve the committed spec, wait for the original process to exit, stage
the preserved file again and rerun the merge serially. A returned session ID is
not command completion. Always resolve the session before another Git mutation.

A second occurrence raced the final HEAD update rather than the index: the merge
left MERGE_HEAD and staged main changes, while the spec commit had moved HEAD.
After the process exits, inspect MERGE_HEAD and the staged/worktree spec, stage
the preserved spec, and finish the pending merge. Check session_id immediately
before allowing any dependent mutation, even inside one orchestration script.
