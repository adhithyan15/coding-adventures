## Unreleased — the retrieval is seated per lesson, and Bengali enters R2

Eighth track through the HL-C313 fix, using the per-lesson seating rule
(HL-C318), and the one where the ceiling and the achieved number disagree most —
for a reason worth stating rather than hiding.

**Seat capacity said 72 of 85 were seatable. Only 34 closed.** The gap is not
budget and not the window. It is that the rule sources retrieval only from WORD
lessons, because a word lesson has a headword and a romanization that make
`say *X*` / `read **X**` a real task. Bengali's R2 debt does not live in word
lessons: of the 51 that remain, **32 were introduced by a writing lesson**, 8 by
a word lesson, 6 by a phrase lesson and 5 by a practice lesson.

Bengali also has a third kind of blocker the other tracks do not: 13 of its 85
misses have **no word lesson anywhere inside their 5-15 window**, because its
word lessons are sparse rather than full. No seating rule reaches those; only a
lesson that does not exist yet would.

### Every number re-measured against the merged tree, not derived

    bengali R2 misses (5-15)                           85 ->  51   (-34)
    bengali R1 misses (1-3)                            38 ->  38   (held)
    bengali R3 misses (20-60)                          88 ->  88   (held)
    bengali R4 misses (80-250)                         14 ->  14   (held)
    bengali reinforcement window misses               225 -> 191
    bengali atoms taught                              154 -> 154   (held)
    bengali atoms never revisited                       5 ->   5   (held)
    bengali lessons                                   139 -> 139   (held)
    forward prerequisites                               0 ->   0   (held)
    forward references                                  4 ->   4   (held)
    script closure violations                         271 -> 271   (held)
    corpus R2 misses                                 4212 -> 3725
    lessons at or over the 300s ceiling                 0 ->   0
    computed seconds, median                          172 -> 172   (held)

25 lessons gained a line; the book carries 25 recall lines, 17 with a read.

Falsified before shipping: reverting `BN-C02-naam` and re-measuring put R2 back
to 54 — the three atoms carried by the three lessons it retrieves.

