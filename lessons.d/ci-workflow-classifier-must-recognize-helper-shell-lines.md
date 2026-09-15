---
category: CI & GitHub Actions
---

# CI workflow classifier must recognize helper shell lines

in toolchain-scoped hunks of `.github/workflows/ci.yml`. Adding `sed`/`rm`/etc. to a Lua-only setup hunk without updating `internal/gitdiff/ci_workflow_test.go` makes the build tool fall back to a full monorepo rebuild.
