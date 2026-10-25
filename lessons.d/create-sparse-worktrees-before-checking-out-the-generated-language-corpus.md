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

On Windows, budget filesystem allocation as well as blob sizes before retrying:
338,660 tracked files totalled 1.15 GiB of content but needed an estimated
2.12 GiB with 4 KiB allocation units, before build output. Removing a verified
merged worktree can therefore still leave too little space for a full checkout.
For a package repair, a sparse selection may omit the generated learning corpus
after verifying that it contains no BUILD targets and is not an input to the
selected validation commands; retain all package and build metadata.
