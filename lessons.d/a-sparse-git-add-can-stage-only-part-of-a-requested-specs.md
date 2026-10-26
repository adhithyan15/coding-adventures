---
category: Repo policy / workflow reminders
---

# A sparse git add can stage only part of a requested specs commit

An explicit add of a new specification outside the sparse pattern declined
that file while staging the other requested files. A later commit still
succeeded. Check the add exit status and staged name list before committing;
add the intended path to the sparse definition or use `git add --sparse`.
Here the unpublished specs commit was immediately amended with the missing
profile before any tests or implementation were written.

The warm VisiCalc receipt repeated this with a pattern allowing specification
Markdown but excluding adjacent JSONL. The narrative staged while both raw
receipts were rejected. Preserve the already-published history, explicitly stage
the two receipts with `--sparse`, and verify all three paths before opening the PR.
In PowerShell, stop on a nonzero `$LASTEXITCODE` after `git add`; a later successful
commit or push does not prove the earlier staging command succeeded.
