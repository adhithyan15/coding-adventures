---
category: CI & GitHub Actions
---

# A brand-new branch/PR can show "no checks reported" (zero workflow runs, not merely queued) for hours

during an account-wide GitHub Actions backlog (many concurrent branches pushing at once saturates the concurrent-job ceiling). Distinguish "queued behind others" from "never triggered" with `gh api repos/<owner>/<repo>/actions/workflows/<workflow-id>/runs?branch=<branch> --jq '.total_count'` — `0` means the push/PR event never even created a run; a nonzero count with `status: queued/pending` means it's just waiting its turn. Watch `gh api repos/<owner>/<repo>/actions/runs --jq '.workflow_runs[] | select(.status != "completed") | .status' | sort | uniq -c` for the account-wide backlog size — once it drains (roughly, once `in_progress` count is >0 and the queued count is dropping), new branches start getting picked up on their own. An empty retrigger commit (`git commit --allow-empty`) does not skip the queue; only waiting does.
