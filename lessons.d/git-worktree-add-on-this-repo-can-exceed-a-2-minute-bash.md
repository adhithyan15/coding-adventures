---
category: Cross-platform & Windows BUILD_windows
---

# `git worktree add` on this repo can exceed a 2-minute Bash timeout

(tens of thousands of tracked files across 4800+ packages) and gets killed mid-checkout, leaving 60k+ files showing as deleted in `git status` and a stale `.git/worktrees/<name>/index.lock`. Fix: confirm no real git process is running (`ps aux | grep git`), `rm -f` the stale `index.lock`, then re-run the checkout with a long timeout: `git checkout HEAD -- .` (pass `timeout: 480000` or similar to the Bash tool). Better: pass a generous timeout to the original `git worktree add` call itself rather than letting it hit the default. **A second symptom of the same root cause**: if the timeout kills a `for`-loop of several `git worktree add` calls mid-loop, the interrupted one shows up in `git worktree list` as `locked` with reason `"initializing"`, not just a stale lock file — `git worktree remove --force` refuses to touch a locked worktree. Fix: `git worktree unlock <path>` first, then `git worktree remove --force <path>`, delete the orphaned branch (`git branch -D <branch>`), and recreate it as its own separate command rather than looping several `git worktree add` calls together.
