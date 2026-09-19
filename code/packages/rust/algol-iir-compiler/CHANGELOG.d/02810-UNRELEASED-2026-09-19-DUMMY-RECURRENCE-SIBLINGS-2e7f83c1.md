## Unreleased - 2026-09-19 - Dummy recurrence siblings

Bounded finite `step`/`until`, `while`, and controlled-scalar recurrence
analysis now skips unlabeled ALGOL dummy statements in compound bodies. Labeled
dummies and every effectful or otherwise unsupported statement remain
conservative.
