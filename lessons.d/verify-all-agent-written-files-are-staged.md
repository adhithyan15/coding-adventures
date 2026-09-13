---
category: CI & GitHub Actions
---

# Verify all agent-written files are staged

Parallel agents may write after the initial `git add`. Run `git status --short` and `git diff --name-only` before committing.
