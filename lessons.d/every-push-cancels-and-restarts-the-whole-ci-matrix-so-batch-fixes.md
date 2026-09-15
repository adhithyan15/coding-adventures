---
category: CI & GitHub Actions
---

# Every push cancels and restarts the whole CI matrix, so batch fixes

2026-09-13.

Six pushes to one branch in quick succession, each a small genuine improvement.
Each one cancelled the in-flight run and started the ~33-job matrix again from
zero, so the branch never got within reach of green and the earlier runs show
as `cancelled` rather than as anything useful.

This is easy to miss because the symptom looks like a different problem. The
PR's check count kept dropping back to 2 while sibling PRs showed 55, which
reads as "my change triggered almost nothing" — a coverage worry — when it
actually meant "the matrix has not finished materialising because it restarted
a minute ago". `gh run list --branch <branch>` shows the cancelled runs and
settles it.

**How to apply.** Once a branch is pushed and CI is running, hold further
polish and land it in ONE follow-up commit rather than a stream. Small
doc-only or comment-only commits are not free: they cost a full matrix restart
each, and they delay the very signal you are waiting on.

The exception is a fix for something actually red. Restarting the matrix to
correct a real failure is the matrix doing its job.
