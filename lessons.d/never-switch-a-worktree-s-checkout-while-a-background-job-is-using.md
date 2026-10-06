---
category: Repo policy / workflow reminders
---

# Never switch a worktree's checkout while a background job is using it

**What happened:** a background loop was regenerating and testing one French A2
tranche after another in a dedicated worktree. While it ran, I wanted to read a test
file as it is on `origin/main`, so I ran `git checkout origin/main` in that same
worktree. The regenerator was halfway through, and it read and wrote a tree that was
part tranche and part main. The result was a spurious diff: 270 "new" narration-hash
shards and a reverted chapter title. The loop's next `git checkout` then aborted
because those files would have been overwritten.

Nothing reached a commit. The stray output went into a named `git stash`, and both
tranches were re-validated from a clean checkout. They were clean.

**What to do differently:**

1. To read a file at another revision, do not move HEAD. Use `git show <rev>:<path>`,
   `git grep <pattern> <rev>`, or `git ls-tree <rev>` instead.
2. If you need a whole tree at another revision, add a separate worktree
   (`git worktree add <dir> <rev>`) rather than borrowing one that a job owns.
3. Treat a validation run that overlapped any change to its worktree as void, and
   re-run it.
