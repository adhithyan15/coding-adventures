---
category: Repo policy / workflow reminders
---

# A Git status parser must preserve the first line's status columns

Porcelain status records start with two meaningful status columns, each of
which may be a space. Calling `.strip()` on the whole command output removes a
leading space from the first record and shifts its path into the status field.
A guard can then report a clean worktree as dirty, or misclassify a real
change. Preserve the raw output and split only on line boundaries, removing at
most the final newline (for example, `stdout.rstrip("\r\n").splitlines()`).
Parse the two status columns before the path; do not trim the record itself.
Keep a regression case whose first status column is a space.
