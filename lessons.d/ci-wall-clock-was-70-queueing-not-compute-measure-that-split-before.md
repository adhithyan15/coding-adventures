---
category: CI & GitHub Actions
---

# CI wall-clock was ~70% queueing, not compute — measure that split before optimizing anything

Across 35 successful PR runs of `ci.yml`: ~91 min mean wall-clock against ~48 min of total execution across 16 jobs, with `detect` waiting a **median 43 minutes for a runner** before it even started. The instinct is to make slow jobs faster; the actual lever was to stop *starting* jobs a change does not need, because every run claimed 16 concurrent slots from a saturated account-wide ceiling and so lengthened the queue for every other run in flight. Near saturation this is superlinear — removing jobs helps every PR, not just the one you edited. Get the numbers with `gh api repos/<owner>/<repo>/actions/runs/<id>/jobs` and diff `created_at` → `started_at` (queue) against `started_at` → `completed_at` (execution); do NOT reason from the run's total duration, which hides the split entirely. Corollary, and it is counterintuitive: **sharding a PR build makes things WORSE while the queue is the bottleneck** — build execution was only ~8 min median, so sharding N ways multiplies runner demand and re-pays toolchain install N times to save a few minutes of compute. Free the capacity first, re-measure, then shard.
