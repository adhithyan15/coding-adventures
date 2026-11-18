---
category: CI & GitHub Actions
---

# A nested package helper import must resolve from the package root

An optional parser test script imports the existing TypeScript SAT solver from
another package. Counting path segments from the repository root instead of
the script's package directory pointed Node at `code/typescript/...`, and the
module load failed before the SAT check began. Resolve cross-package imports
from the actual script location and run the exact documented command after
building the dependency.
