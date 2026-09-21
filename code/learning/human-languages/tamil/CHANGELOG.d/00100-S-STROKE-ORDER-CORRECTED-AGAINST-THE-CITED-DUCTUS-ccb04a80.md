## ம's stroke order corrected against the cited ductus

Two places in the repo described how **ம** is written, and they disagreed about
the one thing a static picture cannot show: **where the hand leaves the paper.**

- `tamil.json` listed three steps — *left vertical, bottom horizontal, right
  arch* — with no citation. Rendered as a numbered list under *"Write it — stroke
  order"*, that reads as three strokes and **two pen lifts.**
- Language Ladder's `strokes.ts` carries an authored pen path for ம: **one
  unbroken stroke** of five joined segments — **zero lifts** — cited to
  Radhakrishnan's *Tamil Script Learners Manual* (Appendix I, Frame 1, UT Austin)
  and checked mechanically against the font outline.

They were **not** in conflict about the ink. The prose was a coarser, three-way
naming of the same left → bottom → right motion, and its order matched. They
conflicted about **pen lifts** — and only the ductus had evidence. So the cited
source wins:

- `strokeOrder` is now the five cited movements, each worded **"without
  lifting"**, ending "*and only now lift*".
- `strokeOrderNote` states the claim in the heading the app renders: *one
  unbroken stroke — five movements, no pen lift*, with the citation.
- New `penLifts: 0` and `strokeOrderSource` fields record the verified claim and
  its provenance as data rather than as prose.
- The letter's `notes` now say outright that the five movements are parts of one
  pen-down run, not five strokes.

The **other ten** Tamil letters have no authored pen path, so nothing verifies
where their pen lifts. Their steps are unchanged — inventing lifts would repeat
the mistake in the other direction — and the file's script-level `notes` now
tell the next author to read them as part order only. Backlog item **HL-C19**
tracks verifying all 190 prose stroke orders across the nine scripts.
