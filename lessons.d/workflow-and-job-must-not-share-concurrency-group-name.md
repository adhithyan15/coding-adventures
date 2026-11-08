---
category: CI & GitHub Actions
---

# Workflow and job must not share concurrency group name

When a workflow defines a top-level `concurrency.group` (e.g. `gh-pages-deployment`) and an internal job inside that workflow also defines `concurrency.group` with the identical group name, GitHub Actions detects a self-deadlock between the top-level workflow run and the job. The job is immediately aborted with `conclusion: "failure"`, `steps: []`, and 0s runtime without ever running on a runner or printing logs (`log not found: <job-id>`).

In multi-job deploy workflows where only the deploy/publish job should serialize (such as `deploy-root.yml` and `deploy-blog.yml`), keep the workflow-level concurrency scoped to the workflow and branch/PR (e.g. `${{ github.event_name == 'pull_request' && format('forme-root-pr-{0}', github.event.pull_request.number) || format('forme-root-{0}', github.ref) }}`), and only assign the shared serializing concurrency group (e.g. `gh-pages-deployment`) to the specific publication job.
