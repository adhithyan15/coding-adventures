## German chapter 20 becomes two chapters, and the corpus has no hand-written chapter left

`ch20-colours.tex` is now generated, as **chapter 20, *The Colour German Lent
Away***, and **chapter 21, *An Ancient Cousin and a Second Emigrant***. Old
chapters 21–43 renumber to 22–44.

**German's hand-written chapters: 1 → 0. The corpus's: 1 → 0.** Every chapter of
every book in this repository is now generated from lesson markdown.

### Sizing it four ways

| instrument | answer |
|---|---|
| `handwritten_parity.py` | gap of **9** — the largest that remained |
| `grep -l '^chapter: 20$'` | **2 lessons** |
| the `.tex` | **2 sections + 2 subsections**, four tabular environments, 154 lines |
| taught-form census | *schwarz*, *weiß*, *rot*, *blau*, *blank* — **5 forms, all headwords nowhere** |

**14 atoms against a ceiling of 12**, so two chapters at the `.tex`'s own section
seam: **8 and 6**.

### The right chapter to end on

Nineteen chapters of this book are about German buying words from Rome — the
clock, the twelve months, the wine. This one is the reverse, and the `.tex` knew
it: German's colour words are all home-grown, and **two of them emigrated**.

***blank***, German for "shining, polished," became the word for **white** in
French, Italian and Portuguese — *blanc*, *bianco*, *branco* — pushing Latin
*albus* out of everyday use. ***blau*** went into French as *bleu*, and English
then borrowed **blue** back from French rather than keeping the Germanic word it
had inherited itself.

And ***rot*** is a third relationship again, one the book has not shown before:
Latin *ruber* is its **cousin, not its source**. The resemblance is descent from
far enough back that neither language borrowed from the other — which is only
legible because the reader has spent the whole book learning to tell a loan from
an inheritance by how far the words have drifted.

### The asterisk hazard, checked end to end rather than flagged

The four reconstructed forms — *\*swartaz*, *\*hwītaz*, *\*raudaz*, *\*blēwaz* —
were the standing exposure to LaTeX eating an asterisk, because `\\` followed by
`*` is the starred line break and LaTeX skips whitespace when hunting for it.

It was checked in three places rather than assumed:

1. **What the form renders as.** Chapter 22's shipped `GE-C14-habere` already
   writes `\**habjaną*`, and its generated `.tex` holds `*\emph{habjaną}` — a
   literal asterisk followed by a command. So the exposure is real in shape.
2. **Where they sit.** `grep '^| *\\\*'` over all twelve drafts returns nothing:
   every reconstructed form is in the **second** cell of its row, and a row's
   first cell is what follows `\\`.
3. **The compiled PDF.** Every occurrence that should carry an asterisk carries
   one. The three that do not are the two chapter-payoff summaries and one
   quoted mention, none of which were marked in the source either.

### Counters, re-measured against the merged tree

| counter | before | after |
|---|---|---|
| German hand-written chapters | 1 | **0** |
| corpus hand-written chapters | 1 | **0** |
| `handwritten_parity.py` german | 9 | **0** blocks at risk |
| German lessons (schema-v2) | 285 | **297** |
| atoms taught | 454 | **468** |
| atom-measurement-blind lessons | 2 | **0** |
| chapters over the 12-atom budget | 5 | **5** |
| culture claims | 29 | **32** |
| atoms never revisited | 81 | **80** |
| forward references | 27 | **25** |
| corpus narration refusals | 44 | **44** |

`handwritten_parity.py` now examines zero chapters across zero tracks and reports
zero blocks at risk. It is the counter this whole programme existed to drive to
nothing, and it is there.

