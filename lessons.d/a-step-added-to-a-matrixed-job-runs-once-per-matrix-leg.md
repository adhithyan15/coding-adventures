---
category: CI & GitHub Actions
---

# A step added to a matrixed job runs once per matrix leg, so a one-off acceptance lane in main's sharded build ran fifteen times

The Linux Mosaic runtime acceptance (the Qt, Flutter and Compose round
trips, the Android APK builds and the emulator launch) was added as steps
of ci.yml's `build` job. On a pull request that job is one Linux runner, so
the steps ran once. On main, `build` is a 15-shard matrix, and its gates are
`needs.detect.outputs.*` flags that are the same on every leg. Every shard
ran the whole lane before starting its own package build.

On the green main run 39369343109 (2026-10-20), these steps took about 35
of each shard's ~93 minutes. That is ~520 runner-minutes per merge, plus
~250 more of other per-shard setup. Only 45% of shard time was the package
build. The lane also left ~18 GB of APKs and Gradle and Flutter projects in
`RUNNER_TEMP` on every shard. The shards that died with "No space left on
device" died late, in Rust archive writes, with all of that still on disk.
On a runner cap of ~20 concurrent jobs, the duplicated work also kept PR
jobs queued for hours.

Nothing failed, so nothing pointed at it. The cost only shows up when you
add up step time across the legs of one run. The job API's `steps` timings
do that in a few lines.

Fix: the lane moved to its own job, `mosaic-linux-runtime`, which needs
`detect` and runs once per workflow run beside `build`. `CI gate` requires
it. A test in `test_mosaic_compose_runtime_ci_acceptance.py` pins the steps
to that job and keeps them out of `build`.

Next time:
- Before adding a step to `build`, ask whether its result depends on which
  shard runs it. If not (an acceptance lane, a one-off oracle, an app
  build), give it a job of its own.
- To audit a matrixed job, sum each step's duration over every leg of one
  run and compare that to the step that is the reason the matrix exists.
- A test that finds a step with `workflow.index(...)` or `rfind(...)` is
  anchored to file order, not to a job. Adding a job can silently move what
  it checks. `test_ocaml_toolchain_lock.py` mutates the *last* pinned
  `actions/checkout`, and a new job placed after `build` took that position.
