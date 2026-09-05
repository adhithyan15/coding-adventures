## Unreleased — the chapter boundary now reaches back far enough to enter R2

Third track through the HL-C313 fix, after Telugu and Sanskrit. A
one-new-word-per-lesson chapter retrieves each word only from the lessons that
follow it inside the chapter, so the largest distance the shape can produce is
4 and R2 (5-15, *"first real retrieval"*) is unreachable. Chapters 34-80 now
carry, in 140 word lessons, one `[YOU RECALL: ...]` task reaching back to an
earlier chapter position for position, at a distance computed against the actual
reading order and rejected outside 5-14.

    - [YOU RECALL: say *parivār*, then read **दीया**]
    - [YOU RECALL: read **बेटी**, then say *kapṛā*]

### Hindi broke three assumptions Telugu and Sanskrit never tested

**The reach is not always two chapters back.** Chapter 59 is eleven consecutive
writing lessons, so chapter 60 sits sixteen lessons past chapter 58 — outside the
window in one step. The rule now walks forward from the nearest candidate and
takes the first chapter whose *every* pair lands inside, which is two chapters
back almost everywhere and three or four where a script block interrupts.
Chapter 58 has no valid target at all and is reported uncovered rather than
given a line at distance 16 that the measurement would not credit.

**The five-minute ceiling is real budget here.** Telugu and Sanskrit word
lessons compute at 110-160 seconds. Hindi chapters 34-39 compute at 268-299
against a 300 ceiling, and a single four-second retrieval line turned
`HI-C36-kursi`, `HI-C38-pet` and `HI-C38-daant` red on the first run. The line
was not shortened to fit: placement moves instead, to a lesson in the same
chapter that has budget and still lands in the window, and where no such seat
exists the chapter is reported uncovered. Chapters 34 and 36 are uncovered for
this reason — thirteen atoms — and HL-C317 records the measurement and the split
that would release them.

**"read **X**" is a claim about the reader, and Hindi can check it.** Asking for
a word whose letters have not been taught yet is exactly the defect
`script-closure.ts` measures, and the first draft turned fifteen clean Hindi
lessons into closure violations (271 -> 286). Telugu and Sanskrit have no
writing lessons at all, so every lesson there already carried that debt and the
same edit changed nothing — the gate simply never fired. A word is now offered
to *read* only where every one of its glyphs has already been taught by an
earlier script lesson, and offered to *say* otherwise: 131 of the 154 recall
lines in the book still carry a read, and 23 are spoken on both sides because
the letters have not arrived.

### Every number re-measured against the merged tree, not derived

    hindi R2 misses (5-15, "first real retrieval")   355 -> 180   (-175)
    hindi R1 misses (1-3)                            145 -> 145   (held)
    hindi R3 misses (20-60)                          364 -> 364   (held)
    hindi R4 misses (80-250)                         291 -> 291   (held)
    hindi reinforcement window misses               1155 -> 980
    hindi atoms taught                               409 -> 409   (held)
    hindi atoms never revisited                       78 ->  70   (improved)
    hindi lessons                                    343 -> 343   (held)
    forward prerequisites                              0 ->   0   (held)
    forward references                                11 ->  11   (held)
    script closure violations                        271 -> 271   (held)
    corpus R2 misses                                4524 -> 4349
    lessons at or over the 300s ceiling                0 ->   0
    computed seconds, median of ch34-81              121 -> 124

The derivation was falsified before shipping: reverting the single lesson
`HI-C40-here` and re-measuring put R2 up by two — `HI-C37-khaana` introduces two
atoms and that lesson is their only retrieval inside the window.

The corpus line was re-measured after Sanskrit's own fix merged underneath this
branch; the Hindi columns did not move, because only Hindi lessons changed here,
but the corpus pair did and was taken from the tree rather than carried forward.

### One prediction this branch made and then contradicted

HL-C316 cautioned that Hindi's total "will not fall the way Telugu's and
Sanskrit's did", because Hindi's non-attributable debt is 71 `never` and 45
`onlyLate` against only 58 `onlyEarly`. It fell 49%, against Telugu's 46% and
Sanskrit's 52%. The caution was about the wrong denominator: those buckets
describe what is left *outside* the one-per-lesson chapters, and Hindi's
chapters 34-80 hold more attributable atoms than reading the remainder
suggested. What the caution got right is where the survivors are: of the 180
that remain, **140 sit in chapters 1-33**, which are phrase, writing, etymology
and practice lessons rather than this shape and need their own reading. The
other 40 are named rather than absorbed —

    ch34:8  ch36:5   the duration ceiling, HL-C317
    ch58:5           the eleven-lesson writing block at chapter 59
    ch59:5  ch67:7   script atoms, which a word lesson cannot recall
    ch35:3  ch37:2  ch40:1  ch41:1  ch74:3
                     phrase and grammar atoms inside covered chapters


