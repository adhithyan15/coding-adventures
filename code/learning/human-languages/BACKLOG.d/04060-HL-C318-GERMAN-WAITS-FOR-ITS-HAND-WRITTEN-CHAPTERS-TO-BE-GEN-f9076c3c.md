## HL-C318 — German waits for its hand-written chapters to be generated before the R2 reach touches it

HL-C316's corrected apply-list puts German in scope: 107 of its 206 R2 misses
are `onlyEarly` — every retrieval packed into distances 1 to 4 — which is
exactly what the chapter-boundary reach fixes. That is still true and the work
is still worth doing.

**It must not be picked up yet.** German holds the last seven hand-written
chapters in the repo and an agent is working through them bottom-up, splitting
and renumbering as it goes. Adding a retrieval line to a lesson whose id,
sequence and chapter are about to change means either a conflict or, worse, a
line that merges cleanly onto a lesson that has since moved and now sits at a
distance nobody measured.

The ordering constraint, in one line: **generate German's remaining hand-written
chapters first, then measure, then reach.** The measurement has to be redone
after the split in any case — every distance in this fix is computed from
reading position, and a split changes reading position for everything after it.

The same caution applies to any track mid-migration. French chapters 9-16 were
under the same constraint while they were being generated; they are finished and
French is now free, with 132 `onlyEarly` atoms of its own.
