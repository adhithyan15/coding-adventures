# An agent isolated in a `.claude/worktrees/<id>` worktree must NEVER `cd` to the repo's top-level absolute path — it silently lands in a DIFFERENT sibling checkout

**Context:** Started a task already inside a dedicated worktree at
`/Users/.../coding-adventures/.claude/worktrees/agent-<id>` (Bash's default cwd,
confirmed by an early `pwd`). Ran a later command as
`cd /Users/.../coding-adventures && git checkout -b <branch>` out of habit —
`/Users/.../coding-adventures` LOOKS like "the repo root" and IS a valid git
checkout, but it is the **main/shared checkout**, a sibling of the worktree, not
an ancestor or the same directory. The `git checkout -b` succeeded there with no
error (it's a completely valid repo), silently switching the SHARED checkout's
active branch and leaving the actual worktree untouched and still on its
original branch.

**How it was caught:** The Write tool refused a subsequent absolute-path write
under `/Users/.../coding-adventures/code/...` with: "This agent is isolated in
the worktree .../.claude/worktrees/agent-<id>. Edit the worktree copy of this
file instead of the shared-checkout path." — i.e. the write guard, not the git
command, is what flags the mistake. `git`/`find`/`grep`/`Read` all succeed
silently against the wrong checkout because it's a real, valid repo with
(usually) identical file content at the same commit — there is no error to
notice until a Write/Edit call is rejected, or until `git status`/`git log` is
inspected side-by-side in both directories.

**Fix:** In an isolated-worktree session, never `cd` to (or hardcode absolute
paths rooted at) the plain top-level clone path — always use the cwd Bash
already defaults to (the actual worktree path), or explicitly prefix every
absolute path with the worktree's own directory. If a stray `cd` to the wrong
checkout already happened: check `git branch --show-current` and `git status
--short` in BOTH directories, restore the shared checkout's original branch
(`git checkout main` or whatever it was), and delete any stray branch created
there (safe with `git branch -d` if it has no unique commits — verify with `git
log <stray> --oneline` first). Do all actual work (branch, commits, file
writes) only inside the real worktree path. Cross-refs the existing
`git worktree add inherits HEAD` and "default to a fresh worktree" lessons
above — this is the write-side counterpart: even with a correctly-created
worktree, a single absolute-path `cd` habit can still misdirect an entire
session.
