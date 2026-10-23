---
category: Repo policy / workflow reminders
---

# Read GitHub Actions job logs before assuming an archive

The GitHub Actions job-log endpoint can yield the job's plain-text log through
`gh api`. Treating that stream as a ZIP or tar archive makes the diagnostic
pipeline fail before showing the underlying CI error.

Read a job-log response directly first. Add archive extraction only after the
response format has been verified, and keep the raw read bounded with a text
filter such as `tail` when diagnosing a focused failure.
