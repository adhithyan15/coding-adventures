---
category: Repo policy / workflow reminders
---

# Git staging paths are relative to the command working directory

The Warrant source tests were staged with a repository-root path while the
command ran from `code/packages/rust`. Git looked below that directory, so add
and commit failed; Cargo then correctly reported the planned missing source
module. That compiler red did not mean the tests had been committed.

Run staging/commit commands from the repository root, and run Cargo from its
workspace (or pass an explicit manifest). Check each dependent command's exit
status before claiming a commit exists. The corrected root staging committed
the authored tests before implementation; the missing-module red is separate
from the staging mistake.
