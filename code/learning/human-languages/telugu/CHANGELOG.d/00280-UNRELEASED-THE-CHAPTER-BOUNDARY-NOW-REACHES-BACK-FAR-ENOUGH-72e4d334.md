## Unreleased — the chapter boundary now reaches back far enough to enter R2

A one-new-word-per-lesson chapter retrieves each word from the lessons that
follow it *inside* the chapter, and the next chapter's opening lesson reaches
back across the boundary for the last two. Every one of those retrievals lands
at distance 1 to 4. `continuity.ts` calls 5-15 lessons **R2, "first real
retrieval"** — the spacing that actually drives retention — and the shape had
no way to reach it. Not a Telugu mistake: it follows from the geometry of a
five-lesson chapter, and it was found independently on Hindi and on Sanskrit
(HL-C313).

Chapters 48-75 now each carry, in every one of their five lessons, one
`[YOU RECALL: ...]` task naming the word at the **same position two chapters
back** and the word at the same position **one chapter back** — distances 10 and
5, both inside R2. Nothing was reseated, no window was widened, no lesson was
added or removed, and no word was retired: retrieval of an already-taught word
is free, which is the only lever the one-new-item-per-lesson rule leaves.

The five lines of a chapter alternate spoken and read, so a learner walking a
chapter meets both modalities without either being scheduled:

    - [YOU RECALL: say *kṛtajñata*, then read **తలుపు** and say what it means]
    - [YOU RECALL: read **మేలు**, then say *kurcī*]

### Every number re-measured against the merged tree, not derived

    telugu R2 misses (5-15, "first real retrieval")   301 -> 162   (-139)
    telugu R1 misses (1-3)                             79 ->  79   (held)
    telugu R3 misses (20-60)                          353 -> 353   (held)
    telugu R4 misses (80-250)                         281 -> 281   (held)
    telugu reinforcement window misses               1014 -> 875
    telugu atoms taught                               449 -> 449   (held)
    telugu atoms never revisited                       13 ->  13   (held)
    telugu lessons                                    336 -> 336   (held)
    forward references                                 10 ->  10   (held)
    corpus R2 misses                                 4734 -> 4595
    lessons at or over the 300s ceiling (ch46-80)       0 ->   0
    computed seconds, median of ch46-80                127 -> 133

Chapters 46-73 were the whole tranche and every one of their 139 judgeable atoms
now enters R2. What is left in Telugu is not this shape: the two script-letter
atoms interleaved into chapter 51, chapter 79's five atoms (which close when
chapter 81 lands and the same rule reaches them), and 138 misses in chapters
1-45, which predate the tranche shape and need their own reading.

The derivation was falsified before shipping, not just asserted: reverting the
single lesson `TE-C48-teacher` and re-measuring put R2 back to 163 with chapter
46 reappearing at exactly one miss, which is the atom that lesson retrieves.


