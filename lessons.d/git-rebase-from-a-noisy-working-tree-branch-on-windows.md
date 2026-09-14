---
category: Compiler / VM / language pipeline
---

# `git rebase` from a noisy-working-tree branch on Windows

Switching branches in the coding-adventures checkout always surfaces a long list of `D code/programs/kotlin/.../.gradle/...` deletions (Gradle build outputs untracked in some branches, tracked in others). Plain `git rebase origin/main` errors with `cannot rebase: You have unstaged changes`. Fix: `git rebase --autostash origin/main`. Autostash also drops the changes silently if they don't apply cleanly to the rebased branch.
