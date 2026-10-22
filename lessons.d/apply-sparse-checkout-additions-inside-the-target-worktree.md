---
category: Repo policy / workflow reminders
---

# Apply sparse-checkout additions inside the target worktree

`git sparse-checkout add lessons.d` ran in the primary checkout after creating
a sibling worktree, so the target sibling still lacked the lesson shards and a
focused search failed. Sparse-checkout state is worktree-local here: run the
addition with the target worktree as the command directory before using paths
inside it.
