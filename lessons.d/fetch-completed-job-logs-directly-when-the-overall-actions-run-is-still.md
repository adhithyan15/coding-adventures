---
category: CI & GitHub Actions
---

# Fetch completed job logs directly when the overall Actions run is still active

`gh run view --job ... --log-failed` refused logs for a completed failed Windows
job because the overall matrix run was still active. Waiting for unrelated
platforms would have delayed diagnosis of the already terminal package failure.

Read that job's authoritative step conclusion, then fetch its completed logs
through `gh api repos/<owner>/<repo>/actions/jobs/<job-id>/logs`. This endpoint
returned the real strict-Clippy failure while other jobs remained live. Do not
restart or cancel a run merely because one observation route has unavailable logs.
