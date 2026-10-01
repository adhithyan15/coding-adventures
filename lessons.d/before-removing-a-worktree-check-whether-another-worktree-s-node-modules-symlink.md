---
category: Repo policy / workflow reminders
---

# Before removing a worktree, check whether another worktree's node_modules symlinks into it

**What happened:** several curriculum stacks were being built in parallel git
worktrees. Each full corpus checkout is large, and the session's disk allowance
ran out halfway through creating one more. To free space I removed the
worktrees whose branches had already merged, using
`git worktree remove --force`.

The worktree I kept as the "dependency host" did not actually hold its
`node_modules`. Its `node_modules` was a symlink to one of the worktrees I
removed, so every other worktree's symlink now pointed at nothing. The next
`npm run build` failed with `Cannot find type definition file for 'node'`, and
the strict book build failed with it.

Nothing was committed in that state. The fix was `npm ci` in each package
the human-language build touches (`pixel-container`, `paint-instructions`,
`paint-vm`, `paint-vm-svg`, `human-language-data`, `script-ductus`) inside
the host worktree. The tranche that had been mid-validation was then
regenerated and re-validated.

**What to do differently:**

1. Before removing a worktree, resolve every `node_modules` symlink in the
   ones you keep (`readlink -f`). Never remove the real target.
2. Keep one worktree whose `node_modules` are real directories, and point the
   others at it. Do not chain symlink to symlink.
3. Use one worktree per stack, and restack branches inside it, rather than one
   worktree per branch. Removing a worktree keeps its branch.
