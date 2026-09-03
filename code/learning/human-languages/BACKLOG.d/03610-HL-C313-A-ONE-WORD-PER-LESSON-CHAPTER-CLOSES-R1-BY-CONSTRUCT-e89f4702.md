## HL-C313 — a one-word-per-lesson chapter closes R1 by construction and cannot close R2

Measured on Hindi while re-homing the pre-A1 vocabulary tranche to chapters
75-81. The tranche's own reinforcement debt lands almost entirely in **one
window**, and the shape of the chapter is why.

A chapter that teaches one word per lesson retrieves each word from the lessons
that follow it *inside* the chapter — distances 1 to 4 — and the next chapter's
opening lesson reaches back across the boundary for the last two, at distances 1
and 2. Every one of those distances is R1 (1-3). Nothing in the design ever puts
a retrieval at distance 5 to 15, so **R2 is missed by every atom in the tranche
that the book is long enough to judge**, and R3 (20-60) by every atom in the
first three chapters.

The numbers, `measureContinuity` over the Hindi corpus with and without the
tranche's 33 lessons:

    reinforcementWindowMisses   1071 -> 1155   (+84)
      of which this tranche's own 33 atoms          41
      of which pre-existing atoms newly judgeable   43
    R1                           145 ->  145   (+0)
    R2                           324 ->  355   (+31)
    R3                           340 ->  364   (+24)
    R4                           262 ->  291   (+29)

Zero R1. All 41 are R2 and R3. This is not a Hindi fact: Tamil's, Kannada's and
Marathi's tranches are built to the same shape and will measure the same way,
because it follows from the geometry of a five-lesson chapter and not from
anything an author chose.

The fix, if it is worth making, is a second reach: each chapter's **third**
lesson retrieves the previous chapter's first two words, at distances 6 to 8,
and its **fifth** retrieves the previous chapter's third. That lands squarely
inside R2 for every atom of every chapter that has a successor. It is roughly
two extra retrieval bullets and two frontmatter lines per chapter, and it should
be done once across all four tracks rather than piecemeal, so that the tranche
shape carries the property rather than each track re-deriving it.

Deliberately NOT done in the PR that measured it. That PR's subject was a
chapter-number collision; adding a cross-chapter retrieval rule to 18 lessons in
one track would have made the tranche shape diverge from Tamil, Kannada and
Marathi rather than converge with them.

The thin-atom gate the completion plan reads is unaffected and moved the right
way: whole-track atoms revisited fewer than twice fell 132 -> 129, and Hindi's
pre-A1 reinforcement item 54 -> 51, because the R1 reach is what that gate
measures.
