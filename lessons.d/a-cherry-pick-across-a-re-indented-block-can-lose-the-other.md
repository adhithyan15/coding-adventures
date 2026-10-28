---
category: Repo policy / workflow reminders
---

# A cherry-pick across a re-indented block can lose the other side's comments, so diff the result against the new base

**What went wrong.** Journal's Card-rows prototype (Mosaic backlog item 4)
was written before JO-5 (#18364) and cherry-picked onto a `main` that had
JO-5. JO-5 had wrapped the pane in `HostScroll [ timeline-scroll ]` and so
re-indented the whole pane block of `JournalApp.mll`. The cherry-picked
result kept the `HostScroll` code, but lost two pieces of JO-5's text in the
same hunks:

- the header diagram's `HostScroll [ timeline-scroll ]` line;
- the eleven-line comment explaining why the pane scrolls.

Every test still passed, because comments are not checked by anything. It
was found only by reading `git diff origin/main -- <file>` while finishing
the work.

**The fix.** Both were restored by hand, and the comment now also says why
Card rows depend on the scroll (misfit M7).

**Do differently.** After a cherry-pick or rebase across a change that
re-indented the same block, run `git diff <new base> -- <file>` and read
every `-` line. Each one should be something your change meant to remove.
A `-` line that belongs to the other change is a lost edit, even when the
build and tests pass.
