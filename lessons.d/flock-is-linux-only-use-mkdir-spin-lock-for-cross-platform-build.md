---
category: Repo policy / workflow reminders
---

# `flock` is Linux-only — use `mkdir` spin-lock for cross-platform BUILD serialization

`flock /tmp/name.lock sh -c "..."` works on Linux but fails with `sh: flock: command not found` on macOS runners. Replace with: `while ! mkdir /tmp/name.lock 2>/dev/null; do sleep 1; done; (cmd); EC=$?; rmdir /tmp/name.lock 2>/dev/null; exit $EC`. The `mkdir` call is atomic on all POSIX filesystems; the subshell captures the exit code so the lock is always released and the correct status propagates.
