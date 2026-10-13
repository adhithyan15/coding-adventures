---
category: Repo policy / workflow reminders
---

# Run diff check before committing every new specification

The branch's new HL43 specification was committed with an extra blank line at
EOF, and the first post-commit `git diff --check origin/main...HEAD` caught it.
Run `git diff --check` across the full branch before each commit that introduces
a new file, not only across the currently unstaged implementation. Remove the
extra newline, amend or follow up the commit, and repeat the branch-wide check.
