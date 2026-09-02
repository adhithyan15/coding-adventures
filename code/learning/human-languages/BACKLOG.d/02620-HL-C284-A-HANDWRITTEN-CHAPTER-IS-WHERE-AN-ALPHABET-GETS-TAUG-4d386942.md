## HL-C284 — a handwritten chapter is where an alphabet gets taught before anybody notices

Persian Chapter 1 taught nine letters in its opening chapter and no gate said a
word about it, because the chapter was hand-written LaTeX. Every lesson-level
measure the corpus has -- the glyph budget, script closure, the atom ramp, the
five-minute ceiling -- reads LESSONS. A `.tex` file that is not built from
lessons is not measured by any of them, and it does not appear as an exception:
it appears as a pass.

That is worth stating plainly because the reports looked healthy. Persian sat at
`0 lessons above 3 new glyphs` and one script-closure violation while its first
chapter, in four `scriptstep` boxes, named:

    س ل ا م   in salâm
    م ن و     in mamnun
    ب ه       in bale and na

against exactly ONE letter taught anywhere in the chapter (`FA-W00`, alef). Nine
against one, in the first six pages of the book. The owner found this by opening
the book, which is the only instrument that was pointed at it.

**The general claim.** Retiring a handwritten chapter is not primarily a
plumbing change. It is the act of putting a chapter under measurement for the
first time, and the flip should be expected to SURFACE debt rather than to be
clean. Italian and Portuguese each moved `chapters above 12 atoms` up by one on
flip, for the same reason in a different currency: a schema-v1 lesson declares
no atoms and so contributes zero -- not "a little", zero -- to every atom budget
in the corpus.

So the honest expectation for the remaining handwritten chapters is that each
one arrives with a finding attached, and a flip that reports nothing new is the
result worth double-checking.

**What the parity script cannot tell you, and should say so.**
`handwritten_parity.py` scored this chapter at 16 orphaned blocks and was right.
What it could not say is that 4 of the 16 -- the `scriptstep` boxes -- were the
chapter's central defect rather than prose to rescue. Carrying them faithfully
would have been the WRONG answer, and the script's output reads like an
instruction to do exactly that ("carry the missing prose into the lessons
first"). The 13 that were carried and the 3 that were reshaped look identical to
it.

A useful upgrade would be for the report to separate the environments by what
they DO: `rootweb`, `usage` and `checkpoint` are prose with a home, while
`scriptstep` is a claim about the script that closure and the glyph budget can
already check once it lives in a lesson. That is a change to the script and
wants its own PR.

**Still owed on this track.** Persian's remaining closure debt -- 46 violating
lessons across chapters 2 and up, 25 glyphs never taught anywhere -- is
untouched. Chapter 1 is now clean and the ladder above it is not, so the
sequencing question ("where does the Perso-Arabic drizzle actually start?") is
open and is the natural next tranche.
