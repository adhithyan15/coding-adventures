---
category: CI & GitHub Actions
---

# A cancellation-aware required gate must distinguish a superseded workflow from a current-head cancelled dependency

While closing FM-B029, the first draft changed the required roll-up job from
`always()` to `always() && !cancelled()`. That suppresses the misleading red
gate after a newer PR head cancels an obsolete workflow, but it also suppresses
the gate when a dependency on the *current* head is cancelled by the platform
or a human. GitHub accepts a skipped required job as a completed check, so the
shortcut can turn missing evidence into a mergeable PR.

Keep the roll-up fail closed. When cancellation reaches it, compare the event's
validated PR head SHA with a fresh authenticated read of the PR's current head.
Only a mismatch proves the workflow was superseded and may close cleanly. If
the heads still match, inspect the dependency results and fail on `cancelled`
exactly like any other non-success result. A cancellation predicate describes
job state; it does not establish whether the evidence is obsolete.

The `always()` requirement applies to the verification step as well as its job.
An ordinary step-level `if` is implicitly guarded by `success()`, so a cancelled
workflow can skip both a separate classification step and the final verifier.
Keep the authenticated head read and dependency checks in one atomic step with
`if: always()`: API or validation errors then fail closed, a proven superseded
head exits cleanly, and a current-head cancellation reaches the result checks.
