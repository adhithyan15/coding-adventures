---
category: Repo policy / workflow reminders
---

# Stop your own background test run by its PID, never with pkill -f on a pattern another worktree's run also matches

**What happened (Japanese chapters 138-142).** A background job ran the 14
`check:*` gates and then `npx vitest run` in one worktree. A prose fix made
that run stale, so it was stopped with `pkill -f "vitest run"` and
`pkill -f "npm run check"`. Other worktrees on the same machine can be
running the same suite at the same time, and `pkill -f` matches the whole
command line of every process the user owns, so it can kill their runs too.
It also missed what it was aimed at: the gate that was running at that
moment was `node dist/modality-cli.js --check`, which neither pattern
matched, and it was left running as an orphan until it was found with `ps`
and killed by PID.

**Fix.** Find the processes that belong to this worktree and kill only
those:

```
for p in $(pgrep -f "vitest|dist/.*-cli.js"); do
  [ "$(readlink /proc/$p/cwd)" = "$PWD" ] && kill "$p"
done
```

or keep the PID when starting the run (`cmd & echo $! > run.pid`) and kill
that process group.

**Do differently.** Before stopping a run, list the candidates with
`ps`/`pgrep -fa` and check each one's working directory. Never use a
`pkill -f` pattern on a shared machine that is wider than this worktree's
own path. Better still, finish the cheap edits before starting the long
run, so it does not need stopping.
