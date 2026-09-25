---
category: Testing & coverage
---

# Lengthening a track can surface an old atom as under-reinforced, because the gate only lists atoms that miss a judged window

Adding fifty Hindi vocabulary lessons (chapters 106-115) at the END of the
track, each chained so every new atom had two later revisits, still moved the
pre-A1 ladder from one blocker to two: `reinforcement 1`. None of the new atoms
was thin. The thin one was `HI-CONCEPT-C97-MESSAGE-01`, a chapter-105 writing
atom that had exactly one revisit before the change and still had one after.

The cause is in `measureContinuity`: an atom is only pushed onto
`reinforcement` when it MISSES a reinforcement window, and a window is only
judged when the track is long enough to contain it
(`if (at + window.from > last) continue`). The level gate then filters that
list for `revisits < 2`. So an atom near the end of a track with a single
revisit is invisible to the gate until the track grows past its window, and
any tranche appended after it makes it appear.

The fix was one real revisit, not a pin: the chapter-114 invitation lesson now
practises the message-writing atom, with `HI-C97-sandesh` added to its
prerequisites and a sentence about an invitation having a message's three
parts.

**What to do differently:** after appending lessons to a track, list the thin
atoms introduced by the track's LAST few chapters before the tranche
(`measureContinuity(...).reinforcement.filter(d => d.revisits < 2)`), and give
each one a revisit inside the tranche. The level-gate row says only "1 atom";
it does not say the atom is an old one.
