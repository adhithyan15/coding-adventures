---
category: BUILD files & dependency management
---

# Diff-based change detection requires a real diff

Before `./build-tool --diff-base origin/main`, commit your changes (or verify `git diff --name-only origin/main...HEAD` returns the expected set). On hash/cache fallback the tool may attempt a monorepo-scale build — stop and clean artifacts.
