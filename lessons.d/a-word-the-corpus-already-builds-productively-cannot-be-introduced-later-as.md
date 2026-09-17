---
category: Repo policy / workflow reminders
---

# A word the corpus already builds productively cannot be introduced later as a word

**The check before writing a vocabulary lesson is not "is this word taught?" —
it is "can the reader already build it?"**

Chapter 105 of the Hindi track drafted a lesson introducing **सुनिए** as an
idiomatic attention-getter. Nothing introduced it, so it looked like a gap.
`forward-language` went 22 → 23 and named the real situation: chapter 93 prints
that exact word in a reading block, twelve chapters and eighty-three lessons
earlier, and it is not a lump there — that chapter teaches the respectful
**-iye** ending as a rule, so the reader assembles the form on sight.

**Being absent from every `introduces` list is not the same as being absent from
the reader.** A productive rule plus a known stem is teaching, and the atom
ledger does not record it.

**The fix was to withdraw the lesson, not to reword it.** The temptation is to
change the prose so the metric stops complaining. That would have left the
teaching order backwards and the metric right. What was genuinely missing was a
*sense* — *excuse me* rather than *please listen* — and a sense belongs beside
the form it attaches to, not twelve chapters downstream. It went to the backlog
with the placement written down.

**How to run the check cheaply.** Before drafting, grep the corpus body for the
surface form. A hit inside an earlier lesson is the signal; then read that
lesson and ask whether the reader assembles it there or meets it whole. The
gentle-ramp report gives the same answer afterwards through
`measureContinuity(...).forwardReferences`, which prints the earlier lesson, the
later lesson, and how many lessons early — but afterwards means after a draft
has been written around the wrong assumption.

**The same shape has now bitten twice in three chapters.** The previous one was
**सौ**, taught at chapter 104 while chapter 82 already used **सौवाँ** built on
it; the fix there was to move the lesson earlier rather than reword. Both times
the metric was pointing at an ordering defect and both times it would have been
possible to make the number fall without fixing anything.
