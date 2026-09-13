---
category: Repo policy / workflow reminders
---

# STOP and generalise when you find yourself writing the Nth variant of the same helper

During the cas-summation work an agent generated **74 open PRs + 27 already-merged PRs** that added a hand-written grid of `N-Sqrt × M-Log × polynomial` helper functions — one per `(N, M)` pair, up to N=64. The bodies were identical modulo two hardcoded counts. A single generic `_log_sqrt_poly_effective_x2_generic(node, k)` that *counts* factors instead of hardcoding them handles every `(N, M, K)` combination, including cases beyond the grid that silently failed. Symptoms to watch for: functions whose names embed a small integer (`_two_sqrt_six_log_poly_*`), CHANGELOGs listing `Phase N — N-Family`, version bumps far past semver-meaningful (`2.373.0`), tests that just instantiate the same template N times. Whenever a "family" pattern emerges, ask "can the count be a `for` loop?" before writing helper N+1. Cleanup PR for this specific incident: #4545 (Phase 86 — generic log×sqrt×poly recogniser).
