---
category: Repo policy / workflow reminders
---

# Appending lessons at a track's end makes older atoms' reinforcement windows judgeable for the first time

**What happened (Japanese chapter 131).** `measureContinuity` judges a
window only when the track is long enough to hold it
(`at + window.from <= last`). Nine lessons appended after chapter 130 moved
`last` from 733 to 742. That made 21 older (atom, window) slots judgeable for
the first time: R4 for the atoms at positions 654-662, R3 for 714-722, and R2
for 729-731. None of them had a revisit in range, so the first draft raised
the Japanese R2/R3/R4 misses from 459/245/519 to 462/254/528. Nothing earlier
in the track had changed.

The level gate only fails when such an atom also has fewer than two revisits.
This time every one had three, so no test failed. The count still got worse.

**Fix.** Diff `reinforcement` per (atom, window) before and after, as for an
insertion. The new slots form a diagonal: the atom at `last_old + 1 - 80 + k`
needs a revisit at or after `last_old + 1 + k`, and the same holds for R3
with 20. So warm-up line k of the new chapter retrieves one R4 atom at exactly
80 lessons and one R3 atom at exactly 20. The three R2 atoms go in any
warm-up 5 to 15 lessons after them. All 21 slots were served, and the counts
went back to 459/245/519.

**Do differently.** Treat any append as a reinforcement change too. Before
writing the warm-ups, list the newly judgeable slots. Also check that every
NEW atom near the end has at least two later revisits inside the chapter.
Otherwise one missed R1 or R2 makes it "thin", and the level gate drops the
track.
