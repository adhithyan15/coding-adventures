---
category: Repo policy / workflow reminders
---

# Probe a CI planner with temporary commits in a scratch worktree, never in the one that holds uncommitted edits

To check which gates fire for a given change, the build tool's `-emit-plan`
needs a real `git diff base...HEAD`, so I wrote a loop that made a base commit,
appended a line to a probe file, committed, ran the planner and then ran
`git reset --hard` to undo it. I ran it in the worktree that held my
uncommitted `ci.yml` and gate-registry edits. `git add -A` swept those edits
into the throwaway commit and the `reset --hard` dropped it, so the edits
vanished from the working tree. A probe path inside a sparse-excluded
directory also made `git add` refuse, which left the later iterations
comparing against the wrong commits and printing misleading results.

Recovery: nothing was lost, because `git add -A` had put the edits in a
commit object. `git reflog` listed the throwaway `tmp-change` commits, and
`git checkout <that commit> -- <the two files>` restored exactly them. HEAD
was verified equal to `origin/main` and the edits re-counted before going on.

Do differently:

- Run planner probes in a separate scratch worktree (`git worktree add
  --detach`), where `reset --hard` is harmless, and copy in only the file
  under test (here `ci-gates.json`).
- Never use `git add -A` or `git reset --hard` in a loop in a worktree that has
  uncommitted work. Stage named files, or commit the real work first.
- A probe file in a sparse-excluded path needs `git add --sparse -f`; check
  the exit status of each step, not just the final printout.
- Treat results printed after any error in such a loop as invalid and rerun.
