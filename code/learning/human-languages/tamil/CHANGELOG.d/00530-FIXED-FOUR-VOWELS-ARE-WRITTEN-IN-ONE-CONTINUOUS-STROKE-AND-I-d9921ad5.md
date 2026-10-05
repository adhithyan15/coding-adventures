## Fixed — four vowels are written in one continuous stroke, and ஊ in two

இ, ஐ, ஒ and ஓ are now taught as one continuous stroke with no pen lift, and
ஊ as two strokes with one lift. Before, இ, ஒ and ஓ had one lift, ஊ three and
ஐ four. As with the consonants, those lifts came from reading numbered
movements, or the frames of an animation, as separate pen-down runs. In HP
Labs India's online Tamil handwriting data (the LipiTk 4.0 recognizer,
trained on hpl-tamil-iso-char), the share of stored prototypes written as
one stroke is 100% for இ, 99% for ஐ, 99% for ஒ and 97% for ஓ; 93% of the
prototypes of ஊ are two strokes.

- **The letter records** (`data/scripts/tamil.d/letters/`) now have
  `penLifts: 0` (ஊ: 1) and a `strokeOrder` with one step per movement. Every
  step after the first says "without lifting", except ஊ's fourth, which
  starts ள after the one lift. Each `variation` note says what the source
  numbers and how this ductus follows that order, and adds the HP Labs
  figure.
- **இ** follows the print face's one unbroken line. The inner right curve
  runs straight on down one lower diagonal into the lower-left loop; the
  loop's outer side climbs back to the left crossing; the middle bar carries
  right into the lower-right loop; and that loop runs on up the other
  diagonal into the outer left side and the great arch. No movement retraces
  ink. The old path climbed the wrong diagonal and never traced the other one.
- **ஐ** keeps the animation's five parts in order. The print face's spiral
  ends inside at a free end, so the pen now starts there and curls outward
  (the old path curled inward and stopped there). It runs down the central
  upright to its free foot and draws the upright back up, and after the
  lower-left bowl it comes back down the short centre stem into the
  lower-right bowl.
- **ஒ and ஓ.** The large loop ends at the tip of the tail, and the print face
  joins the top of the lower bowl to that tail, so the pen comes back along
  the tail into the bowl.
- **ஊ** is written as Module 17 builds it: உ without lifting, one lift, then
  ள without lifting, in the same six movements as ள itself.
- **எ was reviewed and keeps its one lift.** Frame 5 draws its right upright
  last, upward from its foot. The print face gives that upright and its bar
  an outline of their own and stops the lower foot well short of it, so one
  stroke would have to reverse the manual's order, or run back round the
  bowl, up the left side and along the bar and draw the upright downward.
- **ஐ's citation was checked.** An index of Info-farmer's Commons animations
  numbers the vowels from அ at 2, so *Writing Tamil 10.gif* is ஐ, as cited
  (8 is எ, 9 is ஏ, 11 is ஒ and 12 is ஓ). The citation is unchanged.
- **Lessons.** TA-S127 (இ), TA-S134 (ஐ), TA-S138 (ஊ) and TA-S139 (ஒ) show
  the new steps and pen-lift lines. The எ and ஓ lessons (TA-S112 and
  TA-S126) have no written steps; ஓ's filmstrip changes.
- **Regenerated** from those changes: the five filmstrips (TA-S126, TA-S127,
  TA-S134, TA-S138 and TA-S139), chapters 109, 111 and 112, their narration,
  and the four lessons' modality records.
- `tamil.d/_meta.json` now lists இ, ஐ, ஒ and ஓ with the one-stroke letters
  and ஊ as two strokes, records the எ review, and lists the letters still
  due review (அ, ஆ and ஷ).
