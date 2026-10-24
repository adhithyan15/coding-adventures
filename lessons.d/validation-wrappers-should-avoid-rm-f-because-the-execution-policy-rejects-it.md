---
category: Repo policy / workflow reminders
---

# Validation wrappers should avoid rm -f because the execution policy rejects it even for explicit temporary files

An expected-failure test wrapper tried to clear one explicit `/tmp` log with
`rm -f` before rerunning. The command policy rejected the entire shell script
before any command executed, despite the target being narrow and task-owned.
Create a fresh log with `mktemp` instead, confirm it exists before cleanup, and
use plain `rm` on that exact path. This avoids both policy rejection and broad
or unresolved destructive targets.
