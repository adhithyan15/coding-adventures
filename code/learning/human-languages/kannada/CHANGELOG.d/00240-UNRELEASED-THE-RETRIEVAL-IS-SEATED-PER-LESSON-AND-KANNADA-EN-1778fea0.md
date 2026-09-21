## Unreleased — the retrieval is seated per lesson, and Kannada enters R2

Sixth track through the HL-C313 fix, using the per-lesson seating rule Urdu
forced (HL-C318) rather than the chapter-boundary form the first four tracks
used. Every word lesson is now retrieved by a later word lesson about ten
positions on, at a distance computed against the actual reading order and
rejected outside 5-15.

The seat capacity was counted before anything was written: of Kannada's 302 R2
misses, 280 had a word lesson with duration headroom inside their window, 20 had
seats that were all taken, and 2 had no word lesson in the window at all. **234
of those 280 closed.**

    - [YOU RECALL: say *namaskāra*]

### Every number re-measured against the merged tree, not derived

    kannada R2 misses (5-15, "first real retrieval")   302 ->  68   (-234)
    kannada R1 misses (1-3)                             84 ->  84   (held)
    kannada R3 misses (20-60)                          323 -> 323   (held)
    kannada R4 misses (80-250)                         185 -> 185   (held)
    kannada reinforcement window misses                894 -> 660
    kannada atoms taught                               412 -> 412   (held)
    kannada atoms never revisited                       15 ->   5   (improved)
    kannada lessons                                    303 -> 303   (held)
    forward prerequisites                                0 ->   0   (held)
    forward references                                  13 ->  13   (held)
    script closure violations                          271 -> 271   (held)
    corpus R2 misses                                  4212 -> 3725
    lessons at or over the 300s ceiling                  0 ->   0
    computed seconds, median                           151 -> 153

188 lessons gained a line; the book now carries 290 recall lines, 86 of them
with a read.

The derivation was falsified before shipping: reverting the single lesson
`KA-C02-niinu-niivu` and re-measuring put R2 back to 69, the one atom
`KA-C01-namaskara` contributes.

**The rule has essentially exhausted what it can reach here.** Of the 68 that
remain, only **4 were introduced by a word lesson**; the rest are phrase (32),
writing (29) and grammar (3) atoms. A word lesson has a headword and a
romanization, which is what makes `say *X*` / `read **X**` a real task; a
writing lesson introduces a letter and a phrase lesson a phrase, and both need a
different recall phrasing before they can be scheduled at all.

