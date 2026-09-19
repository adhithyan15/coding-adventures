---
category: Repo policy / workflow reminders
---

# A fresh sparse worktree may hide lessons.d until the path is explicitly added

`git worktree add` inherits the repository's sparse-checkout definition. A
tracked path can therefore exist at `HEAD` while being absent from the new
worktree, and a direct read reports a misleading `No such file or directory`.
Before declaring required guidance missing, inspect `git sparse-checkout list`
and `git ls-tree -r --name-only HEAD`; when the path is tracked but hidden, add
that exact path with `git sparse-checkout add lessons.d` and then read it.
