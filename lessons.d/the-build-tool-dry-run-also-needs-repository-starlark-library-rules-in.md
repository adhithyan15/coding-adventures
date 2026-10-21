---
category: BUILD files & dependency management
---

# The build-tool dry-run also needs repository Starlark library rules in sparse worktrees

After all local Go replacement packages were present, the repository build
selector still panicked while loading
`code/packages/starlark/library-rules/rust_library.star`. A sparse worktree
used for build planning must include `code/packages/starlark` as well as the Go
replacement closure. Verify that rule path before running the selector so a
missing support file is not mistaken for a BUILD-file failure.
