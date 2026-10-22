---
category: Repo policy / workflow reminders
---

# Large multi-hunk patches need current context

A broad specification patch was rejected atomically because one expected
sentence differed from the current file. Inspect the exact surrounding text
and apply smaller section-local patches when a document has several independent
edits.
