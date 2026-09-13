---
category: CI & GitHub Actions
---

# An always-on CI job with no `needs:` is scheduled AHEAD of the job that decides what needs to run

`ci.yml` had eleven unconditional conformance jobs and a `detect` job that computes the build plan. Because the eleven declared no dependency, GitHub started them first and `detect` — the critical path for `build` — sat in the queue behind them. The planner was starved by the jobs it should have been gating. Any job you add unconditionally is not just its own cost; it is a delay on every gated job downstream.
