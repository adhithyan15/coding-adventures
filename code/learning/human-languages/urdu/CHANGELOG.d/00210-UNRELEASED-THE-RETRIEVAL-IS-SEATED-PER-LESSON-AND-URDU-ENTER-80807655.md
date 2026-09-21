## Unreleased — the retrieval is seated per lesson, and Urdu enters R2

Fifth track through the HL-C313 fix, and the first one **measured before it was
scheduled** rather than after. That measurement changed the fix.

### What the measurement said, before anything was written

Urdu is not the vocabulary-tranche shape the first four tracks were. Its
chapters carry one to five word lessons, each lesson introduces two or three
atoms, and its distance histogram has **no cliff at 5** — 111 practice events
already land inside the R2 window, against 202 inside R1. Urdu's 108 R2 misses
are individual atoms that happen to get no hit, not a structural wall.

Two things follow, and both were counted rather than assumed.

**Seat capacity is the binding constraint, not the window.** 28 of Urdu's 56
word lessons have 25 seconds of headroom under the 300-second ceiling, so at
three recalled items each there are 84 seats for 108 atoms. Counting per atom:
70 have a seat, 36 have seats that are all taken, and 2 have no word lesson in
their window at all. **65% is the ceiling here**, and no placement rule beats it.

**The chapter-boundary rule is the wrong granularity.** Both rules were applied
to a clean tree and measured, rather than one measured and the other guessed:

    chapter-granular (chapter N retrieves chapter N-2)   108 -> 65
    per-lesson seating (nearest free seat at ~10)        108 -> 48

Pairing chapter to chapter spends one of a very scarce set of seats on a whole
chapter and starves the next. Seating each source lesson individually fits the
same budget three times better, and on a uniform tranche it makes the same
assignment the chapter rule does — it is a generalisation, not a replacement
(HL-C318).

### The change

23 word lessons in chapters 3-18 gained one `[YOU RECALL: ...]` task naming a
word introduced five to fifteen lessons earlier, preferring ten. The line is not
shortened to fit a budget: a seat is a word lesson that can afford it, and a
source with no affordable seat is reported rather than forced.

Only **3 of the 24 recall lines carry a read**. A word is offered to read only
where every one of its glyphs has already been taught, and Urdu's Nastaliq
ladder arrives late in the book, so most of these retrievals are spoken on both
sides. That is the honest answer rather than a defect: asking a reader to decode
untaught Nastaliq is exactly what `script-closure.ts` counts.

### Every number re-measured against the merged tree, not derived

    urdu R2 misses (5-15, "first real retrieval")   108 ->  48   (-60)
    urdu R1 misses (1-3)                             38 ->  38   (held)
    urdu R3 misses (20-60)                           75 ->  75   (held)
    urdu R4 misses (80-250)                          22 ->  22   (held)
    urdu reinforcement window misses                243 -> 183
    urdu atoms taught                               186 -> 186   (held)
    urdu atoms never revisited                        8 ->   4   (improved)
    urdu lessons                                     89 ->  89   (held)
    forward prerequisites                             0 ->   0   (held)
    forward references                                2 ->   2   (held)
    script closure violations                       271 -> 271   (held)
    corpus R2 misses                               4550 -> 4490
    lessons at or over the 300s ceiling               0 ->   0
    computed seconds, median of ch1-18              219 -> 220

60 of the 70 seatable atoms closed. The remaining ten were lost to greedy
first-come seating rather than to any hard limit, and a better assignment would
recover some of them — nobody should read 48 as the floor.

The derivation was falsified before shipping: reverting the single lesson
`UR-C04-thik` and re-measuring put R2 back to 52, the four atoms carried by the
two lessons it retrieves.

Of the 48 that remain, 17 sit in chapters 1-5 — phrase, writing and practice-mix
lessons rather than word lessons — and 31 in chapters 6-17, where the seats are
full. They close when those lessons are split; HL-C317 has the measurement, and
Urdu is the worst-affected track in the corpus by that count.


