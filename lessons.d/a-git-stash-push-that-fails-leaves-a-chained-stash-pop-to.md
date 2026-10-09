---
category: Repo policy / workflow reminders
---

# A git stash push that fails leaves a chained stash pop to pop somebody else's stash

**What went wrong.** To measure a new corpus check against the lessons as
they were before an edit, I ran, in a git worktree,

    git stash push -q -- 'code/learning/human-languages/*/lessons' && <measure> ; git stash pop -q

The quoted glob did not match as a pathspec, so `stash push` failed and saved
nothing. The `&&` chain stopped, but the `;` did not, so `git stash pop` ran
anyway — and the stash list is shared by every worktree of the repository. It
tried to pop the newest entry, a stash from an unrelated branch with untracked
files ("stray regen output from concurrent checkout"). That pop failed on the
untracked files and kept the entry, so nothing was lost; had the entry held
only tracked changes, it would have been applied to the wrong branch and
dropped from the list.

**The fix.** Nothing to repair: `git stash list` still held the entry and
`git diff` showed none of its files. The measurement was redone without
touching the working tree, from `git archive HEAD <dir> | tar -x -C
<scratch>` and a loader pointed at the scratch root.

**Do differently.** Do not use `git stash` as a temporary "before" view in a
worktree: the stash is repository-wide, and a pop pops whatever is on top.
Export the old tree (`git archive`, `git show HEAD:<path>`, or a second
worktree) and read it from there. If a stash is unavoidable, name it
(`git stash push -m <unique>`), apply it by that name, and never chain `pop`
after a command that can fail without `&&`.
