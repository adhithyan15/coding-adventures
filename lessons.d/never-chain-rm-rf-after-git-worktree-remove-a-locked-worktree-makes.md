---
category: Repo policy / workflow reminders
---

# Never chain rm -rf after git worktree remove; a locked worktree makes remove fail and rm deletes it anyway

A cleanup command ran `git worktree remove <p>; git worktree prune; [ -d <p> ] && rm -rf <p> &`. The worktree was locked by the live Claude session that created it, so `git worktree remove` failed, and the background `rm -rf` deleted the directory anyway. That left a locked, dangling registration. Nothing was lost because the tree was clean and its commits were pushed, but CLAUDE.md forbids removing a locked worktree, and this did exactly that by another route.

The fix: before any cleanup, read `git worktree list --porcelain` and look for `locked` on the entry. Leave a locked worktree alone. Only `rm -rf` a leftover directory that git no longer lists, and never run `rm` after a `remove` that failed. Do not chain the two with `;` or put the `rm` in the background, where a failure of the first step cannot stop it.
