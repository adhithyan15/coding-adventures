## HL-C452-257e139f — numbered Tamil hand-movements are now one continuous stroke for க, த, ந, ர, ள and ற

**Status: OPEN — batch 1 of the Tamil re-fit is done; the other letters are
still to do.** Tamil's pen-lift counts came from reading each numbered arrow
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

**Still open.**

- **Two things to check against the source.** Someone with access to the
  Appendix I PDF should confirm த's numbering and ர's first direction. If
  த's loop is numbered before its bowl, or ர's upright is drawn upward, those
  retraces go away.
- **Batch 2:** ச, ண, ன, ழ and ஞ. Review ங.
- **Batch 3:** எ, ஊ, ஐ, ஒ, ஓ and இ. ஊ's record still describes ள as "the
  familiar three-run order", so it needs updating with this batch.
- **Then:** the grantha ஷ.
