---
category: TypeScript / JavaScript
---

# Check package dependencies in a fresh worktree before invoking local npm test binaries

The first focused test run in a newly created worktree assumed the package's
local `vitest` binary was already installed and failed with `vitest: command
not found`. A worktree does not inherit untracked `node_modules` from another
checkout. Check for the package binary first and run the lockfile-respecting
`npm ci` bootstrap before invoking package scripts in a fresh worktree.
