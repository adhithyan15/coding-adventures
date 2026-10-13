---
category: CI & GitHub Actions
---

# Wire repository script contract tests into an always-running CI step

A new repository-level Python contract suite passed locally but was not added
to any workflow's explicitly enumerated test commands. Green CI would therefore
say nothing about its subprocess, parser, or deterministic-output guarantees.
When adding a standalone repository script, inspect workflow test enumeration
and path filters as part of the implementation, then add its focused suite to
an always-running contract step and a regression that guards that wiring.
