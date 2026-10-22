---
category: Repo policy / workflow reminders
---

# Do not embed prohibited recursive cleanup in validation commands

A temporary-venv validation command embedded an EXIT trap containing recursive
removal, so the command runner rejected the whole script before `mktemp`
created anything. Keep destructive cleanup out of validation invocations;
create an explicit isolated temporary environment and clean it only through an
approved, separately resolved operation when necessary.
