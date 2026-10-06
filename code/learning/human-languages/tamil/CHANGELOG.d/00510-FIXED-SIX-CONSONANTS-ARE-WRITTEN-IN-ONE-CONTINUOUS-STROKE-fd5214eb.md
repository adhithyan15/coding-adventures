## Fixed — six consonants are written in one continuous stroke

க, த, ந, ர, ள and ற are now taught as one continuous stroke with no pen
lift. Before, they had two or three lifts. Those lifts came from reading
each numbered arrow in Radhakrishnan's *Tamil Script Learners Manual*
(Appendix I, "Hand-movements") as a separate pen-down run. The manual
numbers movements and does not say the pen lifts between them. Native
writers draw these letters in one stroke. In HP Labs India's online Tamil
handwriting data (the LipiTk 4.0 recognizer, trained on hpl-tamil-iso-char),
the share of stored prototypes written as one stroke is 88% for க and த,
85% for ந, 90% for ர and ள, and 99% for ற.

- **The letter records** (`data/scripts/tamil.d/letters/`) now have
  `penLifts: 0` and a `strokeOrder` with one step per numbered movement.
  Every step after the first says "without lifting". Each `variation` note
  says what the manual numbers and how this ductus follows that order
  without lifting, and adds the HP Labs figure.
- **Where the path retraces ink.** The fit follows the numbered movements in
  order, so in some letters the pen goes back over ink it has already drawn:
  - க: the pen crosses the middle bar into the right bowl.
  - த: the pen goes round the right bowl twice more, because the manual
    numbers the bowl before the loop.
  - ந: the pen climbs back up the middle upright.
  - ர: the pen climbs back up the left upright.
  - ள and ற: the pen goes down and back up a single shared stem.
- ள now starts inside its loop. ந's components no longer describe its
  middle upright and right bowl as "separately started".
- **Lessons.** TA-S04 (ந), TA-S05 (ற), TA-S06 (க) and TA-S131 (ள) show the
  new steps and "**Pen lifts: 0.** The pen never leaves the paper." TA-S06's
  warm-up no longer says "three pen-down runs". The wrap-up answers in
  TA-S04 and TA-S06 now give **None** as the number of pen lifts.
- **Regenerated** from those changes: the six filmstrips (TA-S04, TA-S05,
  TA-S06, TA-S117, TA-S122 and TA-S131), chapters 8, 10, 13 and 110, their
  narration, and the four lessons' modality records.
- `tamil.d/_meta.json` now states the rule. A numbered arrow, a numbered
  part or a GIF frame is not evidence of a pen lift. A lift needs explicit
  evidence. The letters that still group movements into runs are listed
  there for review.
