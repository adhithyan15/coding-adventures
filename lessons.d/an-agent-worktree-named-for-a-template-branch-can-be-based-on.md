---
category: Repo policy / workflow reminders
---

# An agent worktree named for a template branch can be based on main without the template

The task for converting `rust/gf256` said the isolated worktree was "created
from branch `claude/eager-babbage-rkqy9f`, which contains the not-yet-merged
template work". The worktree's HEAD was in fact the commit that branch forked
from (an `origin/main` commit): `warrant-grammar/src/evidence.rs`,
`VER04h-warrant-derived-evidence.md`, `ct-compare/tests/generated/` and the
manifests were all absent, and `git diff HEAD claude/eager-babbage-rkqy9f` was
55 files. The branch had already merged `origin/main`, so a fast-forward was
safe and did not violate "do not merge origin/main".

**Do instead:** before reading a template, confirm the files it names exist in
the worktree (`ls`, `git log HEAD..<template-branch>`); if the template branch
is strictly ahead, `git merge --ff-only <template-branch>`. Never start
"generalising" from a copy of the pattern you remember.
