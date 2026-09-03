## HL-C318 — seat the retrieval per lesson, not per chapter, and measure the seat capacity before scheduling a track

Telugu, Sanskrit, Hindi and Tamil all took the R2 reach as a CHAPTER rule:
chapter N retrieves chapter N-2, position for position. That worked because all
four have the same vocabulary-tranche shape — four or five word lessons per
chapter, one new word each — so a chapter pairing and a lesson pairing are the
same assignment written two ways.

Urdu is the first track where they come apart, and the chapter rule loses badly.

    urdu, chapter-granular rule    17 lessons written, R2 108 -> 65
    urdu, per-lesson seating       23 lessons written, R2 108 -> 48

Both rules were applied to a clean tree and measured, rather than one being
measured and the other estimated.

Urdu's chapters carry one to five word lessons and each lesson introduces two or
three atoms, so pairing chapter to chapter spends one of a very scarce set of
seats on a whole chapter and starves the next. Seating each source lesson
individually — every word lesson is retrieved by a later word lesson about ten
positions on — fits the same budget three times better.

**The per-lesson form is a generalisation, not a replacement.** On a uniform
tranche the nearest free seat at distance ~10 IS the matching position two
chapters back, so it makes the same assignment. Prefer it for any new track.

### Measure the seat capacity first; it is the binding constraint, not the window

Before scheduling a track, count the seats rather than the debt. For every atom
that misses R2 and whose window the track is long enough to judge, ask whether
any WORD lesson at distance 5-15 has enough duration headroom to carry one more
line (budget + 25s <= the 300s ceiling), capped at three recalled items per
lesson. That upper bound is independent of how the retrievals are assigned:

    track        R2 misses   seatable   no seat in 5-15   seats all full
    kannada         302         280            2               20
    malayalam       278         254            1               23
    bengali          85          72           13                0
    urdu            108          70            2               36

Urdu's ceiling is 65%, and it is capacity — 28 of its 56 word lessons have room,
so 84 seats against 108 atoms. Kannada and Malayalam are near 92% and will
behave like the tracks already done. Bengali's blocker is different again: 13
atoms have no word lesson within the window at all, because its word lessons are
sparse rather than full.

Achieved on Urdu: **60 of the 70 seatable**, the remaining ten lost to greedy
first-come seating rather than to any hard limit. A smarter assignment would
recover some; nobody should assume the greedy number is the ceiling.

### The read/say split is a track property worth knowing in advance

A word is offered to *read* only where every one of its glyphs has already been
taught (HL11 closure). Tamil teaches its letters early and 197 of 239 recall
lines carry a read. Urdu's Nastaliq ladder arrives late and only **3 of 24** do.
That is not a defect in the fix — asking a reader to decode untaught Nastaliq
would be — but it means a track whose script comes late gets a spoken-only
retrieval, and the modality intermix the shape is supposed to provide arrives
only once the letters do.
