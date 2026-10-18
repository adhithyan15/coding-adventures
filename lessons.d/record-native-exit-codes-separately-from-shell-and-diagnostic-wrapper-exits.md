---
category: Repo policy / workflow reminders
---

# Record native exit codes separately from shell and diagnostic wrapper exits

The direct shell whitespace-check invocation returned1. I assumed that was
the native git status in a later diagnostic assertion. Its saved subprocess
capture actually records native exit2, so that assertion failed after saving
the real exit and streams; the diagnostic wrapper returned1. Preserve all
three observations separately and inspect actual native results before
asserting a particular nonzero value. This was an EOF whitespace failure,
not a semantic red, compiler failure or changed validation result.
