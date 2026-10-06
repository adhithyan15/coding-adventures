---
category: Testing & coverage
---

# A book that passes page 999 needs the wider contents page-number box before CI scans its log

The Arabic A2 tranche that took the book past page 999 passed every local check,
including `check-book-compile.sh --strict`, and then failed CI's "Scan book logs
for LaTeX warnings" step: 18 overfull `\hbox`es against a baseline of 0. Every one
was a contents line whose page number had become four digits; `book.cls` gives
the page number a 1.55em box, and "1000" is 4.93pt wider than that.

Bengali, Tamil, German, Sanskrit, Italian, French and others had already met
this and carry the fix in their preambles:
`\renewcommand\@pnumwidth{2.4em}` and `\renewcommand\@tocrmarg{3.4em}`.
Arabic, Persian, Kannada and Telugu did not, because they had never been that
long.

**What to do:** when a tranche will push a track past ~1000 pages, check its
preamble for `\@pnumwidth` first, and add the two lines in the same commit.
And run the scan locally, not only the strict compile, which does not count
warnings: `python3 code/scripts/scan_latex_log_warnings.py --book-root
code/learning/human-languages --baseline
code/learning/human-languages/core/latex-warning-baseline.json` after
`check-book-compile.sh --strict <track>` has left `<track>/book/book.log`.
