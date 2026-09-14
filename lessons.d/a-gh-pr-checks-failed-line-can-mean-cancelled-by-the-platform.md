---
category: CI & GitHub Actions
---

# A `gh pr checks` "FAILED" line can mean "cancelled by the platform," not a real error — check the job `conclusion`, not just its display status

PR #10017 (a pure state-JSON change) showed `build (windows-latest)` and the downstream `CI gate` as failed after ~15 min. `gh api repos/<owner>/<repo>/actions/jobs/<job-id> --jq '{status,conclusion}'` showed `conclusion: "cancelled"` (not `"failure"`) — `ubuntu-latest`/`macos-latest` had already built the identical commit successfully, so the content was never at fault. Fix: `gh run rerun <run-id> --failed` (not a code change, not a new commit) — it passed clean on rerun. Rule: before touching code in response to a CI red X, fetch the job's actual `conclusion` field; `cancelled`/`skipped`/`abandoned` mean "rerun," only `failure` with real log output means "investigate the diff."
