---
category: Repo policy / workflow reminders
---

# Run merged-revision tools from a worktree that contains them

After PR #17427 merged, the deterministic hot-file script was invoked from the
shared checkout, whose current branch predates that script and also contains
unrelated user changes. Python therefore failed before the report could run.
For a tool introduced on recent main, first create or reuse a clean worktree at
the exact merged revision and run the tool there; do not assume the shared
checkout contains current-main files, and never disturb its unrelated state.
