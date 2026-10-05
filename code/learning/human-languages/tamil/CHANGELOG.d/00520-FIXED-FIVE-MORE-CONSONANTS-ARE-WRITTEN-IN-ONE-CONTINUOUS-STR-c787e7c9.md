## Fixed — five more consonants are written in one continuous stroke

ச, ண, ன, ழ and ஞ are now taught as one continuous stroke with no pen lift.
Before, ச, ண and ன had one lift, ழ two and ஞ three. As with the first
batch (க, த, ந, ர, ள and ற), those lifts came from reading the numbered
arrows in Radhakrishnan's *Tamil Script Learners Manual* (Appendix I,
"Hand-movements") as separate pen-down runs. In HP Labs India's online Tamil
handwriting data (the LipiTk 4.0 recognizer, trained on hpl-tamil-iso-char),
the share of stored prototypes written as one stroke is 88% for ச, 93% for
ண, 94% for ன, 97% for ழ and 86% for ஞ.

- **The letter records** (`data/scripts/tamil.d/letters/`) now have
  `penLifts: 0` and a `strokeOrder` with one step per numbered movement.
  Every step after the first says "without lifting". Each `variation` note
  says what the manual numbers and how this ductus follows that order
  without lifting, and adds the HP Labs figure.
- **Where the path retraces ink.** The fit follows the numbered movements in
  order, so in some letters the pen goes back over ink it has already drawn:
  - ச: movement 3 ends at the tip of the middle bar, so the pen comes back
    along the bar to the crossing before it turns round the bowl.
  - ண and ன: the top bar runs out to the right edge, so the pen comes back
    along it to the right upright and draws the upright down.
  - ஞ: the pen comes back along the top bar to the central upright, and
    climbs back up that upright to where the outer bowl leaves it.
  - ழ: the print face joins the lower hook to the stem only at the hook's
    far end, so the pen runs round the hook to its tip and back.
- **Print-face fits.** ழ's low crossbar runs straight into the bowl's foot,
  so movement 3 carries on round the foot and up the bowl's right side, and
  movement 4 comes back over the top into the inner upright. ஞ now starts
  inside its small loop, as its record always said, and spirals out into the
  top bar. ண's and ன's components now describe the right upright as hanging
  from the bar rather than "separate", and ழ's hook is no longer called
  "detached".
- **ங was reviewed and keeps its one lift.** Frame 2 draws its upright as a
  separate, detached part before the body, and the print face gives that
  upright an outline of its own. One stroke would have to reverse the
  manual's order or run back over the whole body.
- **Lessons.** TA-S02 (ண), TA-S03 (ன) and TA-S137 (ஞ) show the new steps
  and "**Pen lifts: 0.** The pen never leaves the paper." TA-S02's and
  TA-S03's component lists match the records. TA-S02's wrap-up answer for
  "how many pen lifts" is now **None**. The ச and ழ lessons (TA-S116 and
  TA-S115) have no written steps, so only their filmstrips change.
- **Regenerated** from those changes: the five filmstrips (TA-S02, TA-S03,
  TA-S115, TA-S116 and TA-S137), chapters 6, 7 and 112, their narration, and
  the three lessons' modality records.
- `tamil.d/_meta.json` now lists ச, ண, ன, ழ and ஞ with the one-stroke
  letters, records the ங review, and lists the letters still due review.
