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

TODO: what went wrong, what the fix was, and what to do differently. Replace this line.
