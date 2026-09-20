## Unreleased — the chapter boundary now reaches back far enough to enter R2

Fourth track through the HL-C313 fix, after Telugu, Sanskrit and Hindi, and the
one the `[YOU RECALL: ...]` shape was borrowed from in the first place. Tamil
already used it to reach a long way back; what it never had was a reach into the
**R2** window — 5 to 15 lessons, the spacing `continuity.ts` calls *"first real
retrieval"*. A one-new-word-per-lesson chapter retrieves each word only from the
lessons that follow it inside the chapter, so the largest distance the shape can
produce is 4.

Chapters 32-81 now carry, in 167 word lessons, one `[YOU RECALL: ...]` task
reaching back to an earlier chapter position for position, at a distance
computed against the actual reading order and rejected outside 5-14. The book
now holds 239 recall lines, 197 of them carrying a read.

    - [YOU RECALL: read **நேரம்**, then say *mariyādai*]
    - [YOU RECALL: say *payaṇam*, then read **சந்தோஷம்**]

### The duration ceiling is the binding constraint here, as it was on Hindi

Tamil chapters 33-39 compute at 273-299 seconds against a 300 ceiling, and five
of those seven chapters have **no** word lesson with 25 seconds to spare. A
retrieval line costs about four seconds, so those chapters cannot take one.

The line was not shortened to fit. Placement moves instead — within a chapter,
to a lesson that has budget and still lands in the window — and where no such
seat exists the chapter is reported uncovered rather than given a line the
measurement would not credit. **Chapters 32 to 36 are uncovered for this reason,
33 atoms**, and they close the moment those lessons are split. HL-C317 carries
the measurement for Tamil, Hindi and the six other Indic tracks, where Urdu
turns out to be worse than either.

`read` versus `say` is decided by whether the reader could actually decode the
word: a word is offered to read only where every one of its glyphs has already
been taught by an earlier script lesson. Tamil teaches its letters early enough
that this rarely bites — 197 of 239 lines carry a read — but the check is the
same one that stopped Hindi turning fifteen clean lessons into script-closure
violations.

### Every number re-measured against the merged tree, not derived

    tamil R2 misses (5-15, "first real retrieval")   336 -> 173   (-163)
    tamil R1 misses (1-3)                            151 -> 151   (held)
    tamil R3 misses (20-60)                          386 -> 386   (held)
    tamil R4 misses (80-250)                         327 -> 327   (held)
    tamil reinforcement window misses               1200 -> 1037
    tamil atoms taught                               477 -> 477   (held)
    tamil atoms never revisited                       55 ->  50   (improved)
    tamil lessons                                    390 -> 390   (held)
    forward prerequisites                              0 ->   0   (held)
    forward references                                 7 ->   7   (held)
    script closure violations                        271 -> 271   (held)
    corpus R2 misses                                4524 -> 4361
    lessons at or over the 300s ceiling                0 ->   0
    computed seconds, median of ch32-81              126 -> 130

The derivation was falsified before shipping: reverting the single lesson
`TA-C40-this` and re-measuring put R2 up by two — `TA-C38-udambu` introduces two
atoms and that lesson is their only retrieval inside the window.

Of the 173 that remain, **126 sit in chapters 1-31**, which are phrase, writing,
etymology and practice lessons rather than this shape. The other 47 are named
rather than absorbed: 33 in chapters 32-36 behind the duration ceiling, and 14
grammar, phrase and script atoms inside covered chapters that no word lesson can
naturally recall.


