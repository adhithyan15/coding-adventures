---
category: Repo policy / workflow reminders
---

# An unsafe cleanup trap rejects the whole command before its safe work starts

A build-tool validation command created a dedicated temporary directory but
put `rm -f` in its exit trap. The command runner rejected the complete script
before `mktemp`, compilation, or validation ran, because force deletion is not
permitted even when the resolved target would have been narrow.

Keep temporary targets explicit and validated, remove the exact generated file
with plain `rm`, and remove its now-empty directory with `rmdir`. Do not attach
a disallowed destructive fallback to otherwise safe work: command validation
happens before execution, so one unsafe clause prevents every preceding step.
