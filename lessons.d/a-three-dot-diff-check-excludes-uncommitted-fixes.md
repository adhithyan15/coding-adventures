---
category: Repo policy / workflow reminders
---

# A three-dot diff check excludes uncommitted fixes

After fixing a committed whitespace defect in the working tree, a repeated
`git diff --check origin/main...HEAD` still reported the committed defect. The
three-dot form compares commits and intentionally excludes unstaged fixes. Use
plain `git diff --check` for working-tree edits, stage and commit the fix, then
use the three-dot branch check to verify the committed series.
