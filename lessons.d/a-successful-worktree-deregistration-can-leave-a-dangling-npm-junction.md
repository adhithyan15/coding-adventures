---
category: Repo policy / workflow reminders
---

# A successful worktree deregistration can leave a dangling npm junction

After exact-head NN49 merge, clean/pushed verification and an archive of ignored
evidence, ordinary `git worktree remove` removed the Git registration but failed
with `Directory not empty`. The Windows residual contained a dangling npm
`node_modules/matrix` junction pointing inside that same removed worktree.
Do not infer that an absent registration means the filesystem directory is gone.

A residual archive reported an error for that dangling junction. Its readable
listing was then compared against every remaining regular file: all 8705 were
present. Keep the archive's nonzero exit explicit; a verified file inventory is
not evidence that the dangling link itself was archived.

The subsequent explicitly audited PowerShell cleanup was rejected by execution
policy before execution. Preserve the residual, report the restriction and do
not retry the deletion through another shell or an equivalent workaround.
Recheck free space separately: an external recovery later provided room to
continue without claiming the rejected cleanup succeeded.
