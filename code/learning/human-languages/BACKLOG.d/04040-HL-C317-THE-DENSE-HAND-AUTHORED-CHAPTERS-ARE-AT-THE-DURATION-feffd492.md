## HL-C317 — the dense hand-authored chapters are at the duration ceiling on Hindi and Tamil alike, and they block retrieval

The Hindi entry above recorded chapters 34-39 sitting at 268-299 seconds against
a 300 ceiling, with six of twenty-two word lessons having room for a sentence.
Tamil measured the same way, in the same band, without either track knowing
about the other. **This is not a Hindi quirk. It is what the hand-authored
chapters look like in every track that has them.**

Effective seconds per word lesson, `estimateLessonDuration` on the merged tree:

    hindi                              tamil
    ch  n   min  max  with room        ch  n   min  max  with room
    34  4   268  279      1            32  6   240  286      3
    35  3   258  275      3            33  4   287  299      0
    36  4   282  299      0            34  4   285  299      0
    37  3   263  296      1            35  4   273  298      1
    38  4   275  298      1            36  4   278  299      0
    39  4   276  298      0            37  3   298  299      0
    40+ …   210  210     all           38  3   295  299      0
                                       39  3   275  294      1
                                       40+ …   210  210     all

Both tracks show the same cliff at the same place: the hand-authored chapters
crowd the ceiling, and everything generated from a lesson tranche afterwards
declares 210 and computes well under it. Tamil is the worse of the two —
chapters 33, 34, 36, 37 and 38 have **no** lesson with 25 seconds to spare.

### What it costs, measured

A retrieval line costs about four computed seconds, so these chapters cannot
take one. The R2 fix therefore left, in an otherwise-covered range:

    hindi   chapters 34 and 36        13 atoms
    tamil   chapters 32, 33, 34, 35, 36   33 atoms

In both tracks the line was NOT shortened to fit. Placement moves within the
chapter to a lesson that has budget and still lands in the window, and where no
such seat exists the chapter is reported uncovered rather than given a line the
measurement would not credit. Forty-six atoms are the price of that honesty, and
they are recoverable the moment these lessons are split.

### The shape of the problem

`HI-C36-kursi` is 464 words and three introduced atoms — feminine gender, the
-ā/-ī lean, and a four-thousand-year Sumerian-to-Arabic etymology — carried by
two grammar-lens blocks, a letters block, five practice bullets and a
three-question wrap-up. `TA-C37-*` and `TA-C38-*` are built the same way. These
are three lessons wearing one lesson's frontmatter, and the chapter policy's
`maxNewAtomsPerLesson` of 3 admits them at exactly the cap.

The split is mechanical because the blocks already mark the seams: the gender
lesson and the etymology lesson are separate teaching points with separate
`hl-knowledge` directives. The corpus has done this before — the Urdu *shukriya*
split recorded in `ramp.test.ts`'s `unmeasurableLessons` history moved that
counter one-for-one — so the procedure and the counters it touches are known.

### Until it is done

**These chapters are closed to new content of any kind**, not only retrieval
lines. Any sentence added to `HI-C36-kursi`, `TA-C37-uur` or their neighbours
fails `ramp.test.ts`'s duration gate, and the failure names the duration rather
than the sentence — so the next agent to touch them loses time to it unless they
read this first.

### It is not confined to these two tracks — measured, not expected

Word lessons with at least 25 seconds of headroom, `estimateLessonDuration` over
the merged tree:

    track        word lessons   with room   chapters with NONE
    kannada          221           208      20, 22, 33, 34
    malayalam        195           178      6, 8, 18, 26, 33, 34, 40
    telugu           259           247      6, 26, 32, 34
    gujarati          82            73
    marathi           65            56
    punjabi           54            38      32, 34, 35
    bengali           57            40      12
    urdu              56            28      7, 8, 11

Two things fall out of this that were not obvious from Hindi alone. The cliff
clusters in the **early hand-authored chapters and again around 32-34**, which
is where several tracks changed authoring style mid-book. And **Urdu is the
worst-affected track in the corpus** — barely half its word lessons have room —
which matters because Urdu's R2 debt is 53 `onlyEarly` and 29 `straddles`
(HL-C316), exactly the shape the chapter-boundary reach fixes. Urdu will hit
this wall harder than Hindi or Tamil did, and it should be measured before it is
scheduled rather than after.
