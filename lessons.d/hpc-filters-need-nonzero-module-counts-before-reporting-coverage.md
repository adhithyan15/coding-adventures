---
category: Testing & coverage
---

# HPC filters need nonzero module counts before reporting coverage

The first external HPC report passed package-qualified TIX names with a slash
to `--include`, which selected no modules and printed `100% (0/0)`. That is no
coverage evidence. Installed HPC help defines `[PACKAGE:][MODULE]`; use the
module name or the documented colon form and assert a nonzero expression
denominator. Package-qualified TIX names also require the parent of the package
MIX directory as `--hpcdir`. Resolve actual artifacts, retain the complete
per-module report, and exclude test modules explicitly before any production
claim. Corrected reports superseded the empty selections before publication.
