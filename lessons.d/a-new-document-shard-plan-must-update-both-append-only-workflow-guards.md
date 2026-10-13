---
category: CI & GitHub Actions
---

# A new document shard plan must update both append-only workflow guards

Registering the Japanese changelog in `DOC_SHARD_PLANS` made three focused
tests fail because the append-only deletion workflow did not yet name its new
shard glob or aggregate path. The plan table is intentionally cross-checked
against both the deletion guard and the aggregate trackedness guard. When
adding any document plan, update both exact whole-line lists and preserve their
shell continuations before treating the migration as implemented.
