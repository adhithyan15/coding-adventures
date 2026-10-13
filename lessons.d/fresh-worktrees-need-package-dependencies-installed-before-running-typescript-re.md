---
category: TypeScript / JavaScript
---

# Fresh worktrees need package dependencies installed before running TypeScript red-green tests

The first focused Script Ductus test in a new worktree failed before collecting
tests because `vitest` was absent. Git worktrees share repository objects, not
ignored `node_modules` directories. Before using a TypeScript test as the red
step, check for the package-local executable and run `npm ci` from the package
that owns its lockfile when it is missing. This distinguishes an intentional
assertion failure from an environment setup failure and keeps the red-green
record meaningful.
