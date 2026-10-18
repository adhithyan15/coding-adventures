## Chapter 1 stops being hand-written LaTeX, and stops naming nine letters (HL-C134)

`persian/book/chapters/ch01-greetings-and-responses.tex` was the track's last
hand-written chapter. The generator skipped it, so nothing it said was reachable
from a lesson and every lesson-level gate reported on it truthfully and
irrelevantly. It is now generated from the six Chapter 1 lessons.

`handwritten_parity.py` scored this chapter at **16 orphaned blocks** —
`checkpoint` x4, `rootweb` x3, `scriptstep` x4, `usage` x5 — in four
environments the generator cannot emit at all. Thirteen of the sixteen are
carried; the other three are the reason this chapter needed rewriting rather
than moving.

**Carried, into blocks the renderer does emit.**

| hand-written | now | where |
|---|---|---|
| `rootweb` x3 | `cousinweb` | `## The word, taken apart` — *s-l-m*, *m-n-n*, and the Indo-Iranian negative behind *na*, all three intact |
| `usage` x5 | `culture` and one writing subsection | `## Why it's said this way` |
| `checkpoint` x4 | the *Before you move on* box | `## Wrap-up Recall` |

**Reshaped, deliberately: `scriptstep`.** These four blocks are what the chapter
was really doing wrong. Between them they name **nine letters** — *س ل ا م*, *م
ن و*, *ب ه* — across four words, in the first chapter of the book, where the
only letter any lesson teaches is **alef**. The script FACTS survive, because
they cost nothing to a reader who knows no letters: Persian runs right to left,
and Persian does not write its short vowels. The letter-by-letter naming and the
joining rule are dropped. That is the change, not a casualty of it.

The checkpoints were re-aimed for the same reason. *"Which visible letter
carries long u in ممنون?"* and *"Which first sound does ن make?"* are questions
about letters nobody has taught. They now ask about meaning, register and the
short-vowel habit — things this chapter genuinely does teach.

**Script closure went from one violation to zero.** `FA-C01-na` printed *bale*
and *mamnun* in Persian script with no romanization beside them, which makes
their glyphs load-bearing; the exposure rule exempts a lesson's own headword and
nothing else. The contrast now leads with the romanization, and the two written
forms stand side by side in the chapter recap, whose headword covers all four
words. The remaining Persian closure debt is downstream of Chapter 2 and
untouched here.

**What the flip needed.** `generate:books` refuses a schema-v1 lesson, and three
of Chapter 1's six were v1: `FA-C01-mamnoon`, `FA-C01-bale`, `FA-C01-na`. Each
is rewritten with hand-authored atoms, warm-ups, and headings the renderer can
classify — the old ones (`## Reuse what you know`, `## Meaning and history`,
`## Build the exchange`, `## Quick recall`, `## Three letters, two hidden
vowels`, `## Use it`, `## One new letter`, `## A very old negative`,
`## Contrast`) all produced `unknown` blocks, which schema v2 rejects. No
renderer feature was invented.

Chapter 1 now measures **10 atoms across 6 lessons**, one new headword each,
inside both the 3-atom lesson budget and the 12-atom chapter budget. Every
lesson is under three minutes of computed content.

