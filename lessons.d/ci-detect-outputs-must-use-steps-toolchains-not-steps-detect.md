---
category: CI & GitHub Actions
---

# CI detect outputs must use `steps.toolchains` (not `steps.detect`)

Adding a new language to CI requires THREE places: `allLanguages` in `main.go`, the detect job `outputs:`, AND `steps.toolchains` normalization (BOTH the `is_main=true` and `else` branches).
