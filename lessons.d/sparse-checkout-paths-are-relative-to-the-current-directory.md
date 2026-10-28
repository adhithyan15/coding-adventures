---
category: Workspace & package metadata
---

# Sparse checkout paths are relative to the current directory

The NN50 local build-tool dry-run initially could not resolve its local Go
replacement modules in the sparse worktree. Adding repository-relative paths
while the process cwd was `code/programs/go/build-tool` did not materialize the
intended root packages, so the retry failed identically. Run sparse updates with
`git -C <exact-worktree-root> sparse-checkout add ...` and inspect the resulting
paths before validation. This is checkout incompleteness, not a Go source bug.
