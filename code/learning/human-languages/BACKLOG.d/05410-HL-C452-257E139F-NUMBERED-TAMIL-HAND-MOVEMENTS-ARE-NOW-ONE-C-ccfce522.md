## HL-C452-257e139f — numbered Tamil hand-movements are now one continuous stroke for க, த, ந, ர, ள and ற

**Status: OPEN — batches 1, 2 and 3 of the Tamil re-fit are done; the other
letters are still to do.** Tamil's pen-lift counts came from reading each numbered arrow
in Radhakrishnan's Appendix I ("Hand-movements") as a separate pen-down run.
HL-C09W (த, four runs), HL-C09Z (ள, three runs) and HL-C10A (ந, three runs)
recorded that reading, and so did HL-C136, HL-C151 and HL-C152. The arrows
number movements and changes of direction inside a letter. The manual does
not say the pen lifts between them. The `tamil.d/_meta.json` note already
said a part list is not a lift count, but nothing applied the same rule to
the sources.

Native writers draw these letters in one stroke. HP Labs India's online
handwriting data (the LipiTk 4.0 Tamil isolated-character recognizer, trained
on hpl-tamil-iso-char) shows the share of stored prototypes written as one
pen-down stroke: க 88%, த 88%, ந 85%, ர 90%, ள 90% and ற 99%.

**Done in this batch.** க, த, ந, ர, ள and ற now have 0 pen lifts. Each
follows the manual's numbered movements in order without lifting. Where a
movement starts somewhere other than where the last one ended, the path
retraces ink it has already drawn. Some changes go beyond merging runs:

- **க.** Movement 3 now stops at the inner crossing, and movement 6 crosses
  the middle bar. Before, movement 3 carried the bar left and movement 6
  restarted at the crossing.
- **ற.** Movement 3 now climbs back up the one middle upright the print face
  has. It used to descend that same stem a second time.
- **ள.** The path now starts inside the loop, because the print face joins
  the loop to the middle upright only at the top.
- **த.** The manual numbers the right bowl (3–4) before the left loop (5–6).
  Following that order without lifting makes the pen retrace the right bowl
  twice.
- **ர.** Movement 1 runs down the left upright, so the pen climbs back up it.

Each record's `variation` note says which of these applies to it. The
`_meta.json` note now names the rule: a numbered arrow, a numbered part or a
GIF frame is not evidence of a lift.

**Done in batch 2.** ச, ண, ன, ழ and ஞ now have 0 pen lifts (before: 1, 1,
1, 2 and 3). In the same HP Labs data their one-stroke shares are ச 88%,
ண 93%, ன 94%, ழ 97% and ஞ 86%. The retraces and print-face fits:

- **ச.** Movement 3 ends at the tip of the middle bar, so movement 4 comes
  back along the bar to the crossing before it turns round the bowl.
- **ண and ன.** The top bar runs out to the right edge, so the last movement
  comes back along it and draws the right upright down.
- **ஞ.** The path now starts inside the small loop, as the record says, and
  spirals out into the top bar. It comes back along the bar to the central
  upright and climbs back up it to the outer bowl.
- **ழ.** The print face's low crossbar runs straight into the bowl's foot,
  so movement 3 carries on round the foot and up the right side and
  movement 4 comes back over the top into the inner upright. The face joins
  the lower hook to the stem only at the hook's far end, so movement 6 runs
  round the hook to its tip and back.

**ங was reviewed and keeps its one lift.** Its one-stroke share is 85%, but
Frame 2 draws the upright as a separate, detached part before the body, and
the print face gives that upright an outline of its own. Joining them in one
stroke would reverse the manual's order or retrace the whole body.

**Done in batch 3.** இ, ஐ, ஒ and ஓ now have 0 pen lifts (before: 1, 4, 1
and 1), and ஊ has 1 (before: 3). In the same HP Labs data the one-stroke
shares are இ 100%, ஐ 99%, ஒ 99% and ஓ 97%, and 93% of the prototypes of ஊ
are two strokes. The fits:

- **இ.** The print face is one unbroken line. Movement 3 goes down the lower
  diagonal that continues the inner right curve, movement 4 climbs the
  lower-left loop's outer side to the left crossing, and movement 6 climbs
  the other diagonal from the lower-right loop into the outer left side.
  Nothing is retraced. The old path climbed the wrong diagonal and left the
  other one untraced.
- **ஐ.** The 13 animation frames sample one movement; they are not five
  runs. The print face's spiral and central upright both have free ends, so
  the pen starts inside the spiral and curls outward (the old path curled
  inward and stopped at that free end), runs down the upright and draws it
  back up, and comes back down the short centre stem into the lower-right
  bowl. The animation gives no direction for the spiral; it does draw the
  upright upward, and the ductus keeps that.
- **ஒ and ஓ.** The large loop ends at the tip of the tail, and the print face
  joins the lower bowl to the tail, so movement 3 comes back along the tail
  into the bowl.
- **ஊ.** உ in one stroke, one lift, then ள in one stroke with ள's own six
  movements, as Module 17 builds it.
- **ஐ's citation was checked.** The learn-tamil project's lesson index
  (`pages/basics/levels.json` in github.com/Kavelin/learn-tamil) embeds
  Info-farmer's animations by number: 2 அ, 3 ஆ, 4 இ, 5 ஈ, 6 உ, 8 எ, 9 ஏ,
  10 ஐ, 11 ஒ and 12 ஓ. So *Writing Tamil 10.gif* is ஐ, as cited, and the
  suggestion that it should be 9 was wrong. The citation is unchanged.

**எ was reviewed and keeps its one lift.** Its one-stroke share is 92%, and
the research note suggested drawing the right upright down. But Frame 5, as
recorded in HL-C10G, draws the upright last and upward from its foot, and no
source in hand says otherwise. In handwriting the lower foot presumably runs
on into that foot; the print face gives the upright and its bar an outline
of their own and stops the lower foot well short of it. One stroke on the
print face would have to reverse the manual's order, or run back round the
bowl, up the left side and along the bar (about 1100 font units of
retracing) and draw the upright downward.

**Still open.**

- **Things to check against the source.** Someone with access to the
  Appendix I PDF should confirm த's numbering and ர's first direction. If
  த's loop is numbered before its bowl, or ர's upright is drawn upward, those
  retraces go away. The same reader should check where Frame 7 starts ழ's
  lower hook (if at the stem, the hook retrace goes away), where Frame 8
  starts ஞ, and whether Frame 2's ங upright is really detached. They should
  also check Frame 5 for எ: whether movement 7 really draws the upright
  upward from its foot, and whether the lower foot runs into it. If the
  upright is drawn down from the bar, எ can be one stroke. The direction of
  ஐ's spiral in the *Writing Tamil 10* animation is also unchecked.
- **ங's existing path** crosses the small gap between the inner stem and the
  low bar (0.971 on ink, just above the 0.97 floor). It should be refitted
  when ங is next touched.
- **Then:** அ and ஆ (one lift each, due review), and the grantha ஷ.
