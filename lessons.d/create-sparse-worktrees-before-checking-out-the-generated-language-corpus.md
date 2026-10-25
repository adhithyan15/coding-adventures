---
category: Repo policy / workflow reminders
---

# Create sparse worktrees before checking out the generated language corpus

A full neural-framework worktree exhausted the remaining disk while checking
out generated human-language book assets. Start with `git worktree add
--no-checkout`, configure cone-mode sparse paths for the actual task, then use
`git read-tree --reset -u HEAD` to populate the initially empty index. That last
step is only appropriate in the new, untouched checkout. An empty index before
population can otherwise make every tracked file appear deleted.

Wait for the failed worktree creation and its rollback to finish before treating
that directory as usable. Do not write work into a checkout still being created;
rollback can remove it. Include new spec/fixture directories in the sparse
selection before committing them: later sparse-checkout changes remove tracked
files outside the selection. Preserve the primary checkout and other tasks.
