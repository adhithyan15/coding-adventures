---
category: Testing & coverage
---

# Install a TypeScript package dependency chain before invoking its local Vitest binary in a fresh worktree

The mock drafter tried the package's direct Vitest command in a new worktree,
but no package dependencies had been installed, so the shell reported that
`vitest` was unavailable. Before running focused TypeScript tests, inspect the
package BUILD file and install its file-based dependency chain in the declared
leaf-to-root order; then invoke the local binary without using an on-demand
package runner that can mutate lockfiles.
