## HL-C10M — Restore the Latin writing-activity directive gate

Refreshing from `origin/main` after Malayalam chillu **ൽ** exposed two new
validation errors from the merged Latin pre-A1 writing tranche: the delayed-copy
and dictation `hl-activity` directives appeared after learner-facing prose even
though the parser contract requires them immediately after the block's
first-line knowledge metadata and before learner copy. Once reachable, both
contracts also duplicated their canonical `answer` in `accepted`, which the
activity compiler correctly rejects as an ambiguous normalized response.

Move only those two directives above the displayed instructions and remove the
redundant accepted-answer copies. This preserves their intended responses and
lesson order while restoring the shared-curriculum validation gate; no learner
prose changes.

