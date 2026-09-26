---
category: Repo policy / workflow reminders
---

# Inserting lessons early in a track moves every later retrieval distance; diff reinforcement misses per atom and window, not per atom

**What went wrong (HL-C443, Marathi and Gujarati runway anchors).** Anchor
words were inserted at the head of the opening chapters. Every lesson after
them moved later, so the distance between an older atom's introduction and its
retrieval grew. Some retrievals that had sat just inside R2 (5-15) or R3
(20-60) fell out of the window. My first before/after check compared the
reinforcement defect lists keyed by atom only. That caught brand-new atoms but
missed old atoms whose `missed` set grew. The Marathi PR went up with two
silent R2 losses (the long-u and vocalic-r signs), and the corpus suite still
passed because Marathi does not pin those windows.

**Fix.** Run `measureContinuity(loadTrackLessons(track)).reinforcement` before
and after, and diff `missed` per (atom, window). For every old atom that lost
a window, add a retrieval at the right distance. A new lesson's warm-up is the
natural place, for example: "From memory, write ર, દ and છ once each." The
new anchor words can also retrieve each other, which gives them R2 and R3.

**Do differently.** Treat any insertion before existing lessons as a
reinforcement change. Report the per-window diff in the PR, with the old-atom
count before and after, and only then move the position pins.
