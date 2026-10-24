---
category: Testing & coverage
---

# Resolve repository scripts from the repository root

A combined package-validation command ran from `code/packages/rust` and tried
to reach `code/scripts/lessons.py` with too many parent traversals. All package
checks passed, but the final repository validation failed before it could run.

Repository-wide scripts should be invoked from the repository root, or their
path should be resolved and checked before appending them to a command running
from a nested package directory. Keep package-local and repository-wide checks
as separate commands when they have different working-directory assumptions.
