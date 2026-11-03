---
category: CI & GitHub Actions
---

# A failed job can be retried only after its workflow run finishes

PR #18849's macOS job had finished with an iPad UI-test failure while Windows
and Mosaic Linux jobs were still running. A targeted rerun request returned
403: "The workflow run containing this job is already running." The failed
job being terminal was not sufficient for GitHub's rerun API.

Record the failed job and expected PR head, then wait for the containing run
to become terminal. Refresh the head and confirm no replacement exists before
requesting the bounded targeted retry. Do not cancel healthy concurrent jobs,
rerun the whole suite, or manufacture a source commit to bypass that wait.
