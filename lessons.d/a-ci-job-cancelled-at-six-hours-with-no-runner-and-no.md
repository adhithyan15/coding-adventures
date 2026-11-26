---
category: CI & GitHub Actions
---

# A CI job cancelled at six hours with no runner and no steps was queue starvation, not a hang

On PR #19909, `build (macos-latest)` showed `cancelled` after 6h39m, while the same probe
finished on Ubuntu in about 4 minutes. GitHub kept no log ("log not found", BlobNotFound),
and it looked like a macOS-only hang in new code, a blocker that would have needed a probe
fix and therefore another long re-recording.

The jobs API showed `runner_name: ""` and zero steps. The job never got a runner: it waited
in the queue for over six hours, while the account's concurrent-job budget was taken by an
8-way recording matrix and other workflows, and was then cancelled. Nothing in the code
was wrong.

What to do differently: before diagnosing a long or cancelled job, read
`gh api repos/<o>/<r>/actions/runs/<id>/attempts/<n>/jobs` and check `runner_name` and the
step count. Empty means queue starvation, so re-run it when runners are free. Do not hunt
for a hang. Also expect macOS jobs to starve while a large Linux matrix (such as
`vos-capacity-full` at `max-parallel: 8`) is running.
