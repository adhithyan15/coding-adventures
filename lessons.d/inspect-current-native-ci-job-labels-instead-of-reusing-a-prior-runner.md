---
category: CI & GitHub Actions
---

# Inspect current native CI job labels instead of reusing a prior runner label

A local native-execution receipt helper reused the earlier Windows job label
`build (windows-latest)`. The refreshed main workflow used
`build (windows-2025)`, so the helper failed closed but could have waited for a
nonexistent job indefinitely. The CI result itself had not failed.

Read platform job names from the exact current-head run before selecting them.
The helper now requires the verified Windows job name explicitly. Match the
actual compiler test step and package result as well as the platform job; do
not treat an OS label, a queued job, or a snapshot command exit as acceptance.
