---
category: Repo policy / workflow reminders
---

# When CI fails for a reason outside the PR, merge the latest origin/main and push instead of waiting for the owner to re-run the job

**What went wrong.** On #20131 the `build (windows-2025)` job failed before any of the PR's
code ran: Chocolatey's feed returned `504 Gateway Timeout` while installing `qpdf`. The
GitHub integration this session uses cannot re-run jobs (`rerun-failed-jobs` returns 403), so
I left one comment explaining the failure and then asked the owner, repeatedly, to press
"Re-run failed jobs". The PR sat blocked for hours on a click it did not need.

**The rule (the owner's standing instruction).** Do not wait for the owner to re-run a job.
The repository moves fast, so `origin/main` almost always has new commits: run
`git fetch origin main && git merge --no-edit origin/main` on the PR branch, run the fast
checks for what changed, and push. The new head starts every workflow again, and the PR is
also tested against current `main`, which a bare re-run would not do.

**Limits.** This is a real merge commit, not an empty commit pushed only to kick CI (still
never allowed). If `main` has truly not moved, say so once and fix or wait on the cause
instead. Still find the root cause first: this applies only when the failure is not the PR's
(an outage, a runner fault, a failure that is red on `main` too). A failure the PR caused
gets a fix, not a re-run.
