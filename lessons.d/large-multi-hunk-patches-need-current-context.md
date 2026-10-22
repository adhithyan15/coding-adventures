---
category: Repo policy / workflow reminders
---

# Large multi-hunk patches need current context

A broad specification patch was rejected atomically because one expected
sentence differed from the current file. Inspect the exact surrounding text
and apply smaller section-local patches when a document has several independent
edits.

The same failure recurred while replacing a large test scaffold: an assumed
tail omitted punctuation present in the file, so an otherwise valid patch was
rejected. Re-read the exact tail and patch helpers, call sites, and test blocks
separately instead of coupling them to one fragile context match.
