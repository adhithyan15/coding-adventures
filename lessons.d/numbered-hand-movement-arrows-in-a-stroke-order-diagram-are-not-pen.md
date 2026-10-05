---
category: Repo policy / workflow reminders
---

# Numbered hand-movement arrows in a stroke-order diagram are not pen lifts

**Context:** the Tamil ductus in `script-ductus` (`src/strokes/tamil/U-*.ts`)
and the letter records in
`code/learning/human-languages/data/scripts/tamil.d/letters/`. They cite
Sankaran Radhakrishnan's *Tamil Script Learners Manual*, Appendix I
("Hand-movements").

**What happened:** the manual draws numbered arrows, one for each direction
segment of a letter. Several tranches (HL-C09W, HL-C09Z, HL-C10A, HL-C136,
HL-C151 and HL-C152) treated the gaps between those arrows as "visible
discontinuities" and grouped the movements into pen-down runs. That gave
த three lifts, and gave க, ந, ர, ள and ற two each. The lessons then taught
"three separate runs" and asked "how many pen lifts? (**Two**)". Native
writers draw all six letters in one stroke. In HP Labs India's online Tamil
handwriting data (the LipiTk 4.0 recognizer, trained on hpl-tamil-iso-char),
85% to 99% of the stored prototypes of each are a single pen-down stroke.
`tamil.d/_meta.json` already warned that a part list is not a lift count.
Nobody applied the same rule to the sources.

**Why:** a stroke-order diagram has to start each arrow a little away from
the last one, or the arrows would overlap and be unreadable. So the gap is
about how the diagram is drawn, not about the pen. The same mistake can be
made with an animated GIF, where the gap between frames is just sampling, and
with a "numbered parts" diagram.

**Fix:** the six letters were re-fitted as one continuous stroke each
(`penLifts: 0`). The fit follows the numbered movements in order and
retraces ink wherever the next movement starts somewhere else. The
`_meta.json` note now states the rule.

**Do differently:** count a pen lift only when a source shows one
explicitly. That means the pen tip jumping in an animation, a second start
dot, or a written instruction to lift. Arrows, numbers and frames never
count. If a fitted route has to retrace a lot of ink to keep the numbered
order, check the numbering against the source again before you add lifts.
