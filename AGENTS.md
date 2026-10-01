# Agent Instructions

Follow `CLAUDE.md` at the repo root: it is the single source of truth for working
principles, repo standards, workflow and the build system, and it applies to every
coding agent here, not just Claude Code.

## Worktrees: clean up after your PR merges

Do not leave worktrees behind. When the PR for your branch merges (or is closed):

1. Confirm `git status --porcelain` is empty and every commit is on a remote branch.
2. From the main checkout, run `git worktree remove <path>`, then `git worktree prune`,
   then `git branch -d <branch>`.
3. Do not use `--force` unless the only dirt is untracked build output you have
   confirmed is regenerable.
4. Never remove a worktree whose PR is still open, that is `locked`, or that another
   session is using.

See the "Clean up your worktree once its PR merges" bullet in the Workflow section of
`CLAUDE.md` for the full rule.
