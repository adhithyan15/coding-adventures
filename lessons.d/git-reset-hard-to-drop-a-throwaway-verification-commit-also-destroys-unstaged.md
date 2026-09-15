---
category: CI & GitHub Actions
---

# `git reset --hard` to drop a throwaway verification commit also destroys unstaged work in the same tree

Doing end-to-end verification by committing a scratch change, running the tool against `HEAD~1`, then `git reset --hard HEAD~1` silently threw away five uncommitted security fixes: they were never staged, so the scratch commit did not contain them and the reset wiped them with everything else. Same family as the `git checkout -- $(git diff --name-only)` footgun. **Commit (or `git stash`) real work BEFORE any verification loop that resets**, and prefer `git stash` + scratch commit + `git reset --hard <saved-sha>` + `git stash pop` when the tree is dirty. `git status --porcelain` returning clean right after a reset you did not expect to be destructive is the tell.
