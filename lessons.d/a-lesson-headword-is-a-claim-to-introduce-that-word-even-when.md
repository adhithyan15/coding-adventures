---
category: Repo policy / workflow reminders
---

# A lesson headword is a claim to introduce that word, even when the lesson is about punctuation

Spanish chapter 426 added a lesson about when the small words inside a surname
take a capital -- `Pedro de la Cruz` against `el senor De la Cruz`. It was given
the headword `de, del y la en los apellidos`, which reads naturally and describes
the lesson accurately.

The gentle-ramp snapshot then moved `forward-language` from 376 to **519**. One
hundred and forty-three new entries, every one of them pointing at a long-merged
lesson, and all of them attributed to this one new lesson.

The cause is that `continuity.ts` treats a lesson's `headword:` as the evidence
that the lesson INTRODUCES that word -- which is correct and is what makes the
forward-reference check provable rather than a guess. A headword of `de, del y
la` therefore claims the lesson introduces **de**, one of the commonest words in
Spanish, taught back in chapter 9 as `soy de`. Every earlier lesson using it was
now using a word "taught later".

So this was a **real defect in the lesson**, not a false positive. The two
earlier entries in this backlog (HL-C378, HL-C379) record genuine detector noise
from homographs and homonyms, and the temptation after writing those is to read
the next `forward-language` rise the same way. A jump of 143 is not that shape:
noise arrives in ones and fives.

The fix was one line. The headword became `la minuscula inicial` -- which is also
the inventory point's own label -- and the count went straight back to 376, zero
new forward references. Nothing about the teaching changed.

Two things to do differently. **Choose a headword that names what the lesson
teaches, not what it discusses.** A lesson about the punctuation of a list should
not be headworded with the list. And **when a snapshot number jumps by an order
of magnitude more than the tranche's size, read the entries before writing the
explanation** -- group them by `taughtBy`, which points straight at the lesson
responsible.
