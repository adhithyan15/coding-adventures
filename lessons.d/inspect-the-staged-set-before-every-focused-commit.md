---
category: Repo policy / workflow reminders
---

# Inspect the staged set before every focused commit

`git rm --cached` had already staged the Japanese monolith deletion. A later
commit intended only for lesson files therefore included that deletion, and the
following `git add` failed because it named a path that no longer existed in
the working tree. Before every focused commit, run `git diff --cached --name-status`
and verify the complete staged set, including changes staged by earlier Git
commands. Once a deletion is already committed, do not name its absent path in
the next `git add`; complete the remaining migration in a follow-up commit.
