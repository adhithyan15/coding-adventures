---
category: CI & GitHub Actions
---

# Setup-job failures (action archive download, `Failed to download archive`)

Read the exact failed step before changing package code. A failure in `Set up
job` or `Prepare all required actions` before checkout has not exercised the
package. Distinguish an unavailable archive, an invalid action ref and a later
build/test failure from the actual log and current upstream evidence.

For Closure carrier9d787797, CodeQL Go, macOS build and metadata jobs failed
downloading the same setup-go archive. The exact upstream commit existed and
a direct archive HEAD returned200; neither proves a hosted job passed. One
targeted Go retry after terminal failure genuinely completed CodeQL attempt3.
The macOS and metadata failures remained recorded, with no acceptance inferred.

Preserve exact-head job logs and annotations. Completed job logs can be read
through the Actions job-logs API while the overall workflow is active, even
when the CLI run-log command refuses. Retry only the evidenced failed job
after its workflow finishes, record the request separately from its terminal
result, and avoid repeated blind retries or action downgrades. Keep required
CI and package tests intact; source changes require their own validation.
