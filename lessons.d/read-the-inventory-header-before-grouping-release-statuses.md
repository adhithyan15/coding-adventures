---
category: Testing & coverage
---

# Read the inventory header before grouping release statuses

An audit grouped the Perl release CSV by a guessed `status` property. The
header names it `grammar_status`; PowerShell silently produced one unnamed
776-row group for the missing property. Reading the header and indexing the
actual column yielded 116 partial and 660 pending entries, with none complete.
Check the header before grouping and use an explicit keyed lookup that fails
for an absent column. An empty or unnamed group is a failed audit, not evidence
that every release has the same status.
