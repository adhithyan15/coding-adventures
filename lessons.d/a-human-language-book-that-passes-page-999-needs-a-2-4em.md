---
category: Testing & coverage
---

# A human-language book that passes page 999 needs a 2.4em TOC page-number box

The Gujarati book grew from about 900 pages to about 1,880 with the A2
vocabulary, and the strict compile then reported 625 overfull boxes. Every
one was a contents line whose four-digit page number ("1000" is 4.93pt
wider) did not fit LaTeX's default 1.55em `\@pnumwidth` box. Twenty other
books' preambles already set `\@pnumwidth{2.4em}` and `\@tocrmarg{3.4em}`,
each added when that book crossed page 999. Gujarati's never had.

This is a real overflow, not a warning to suppress, so the documented
preamble fix is the right one: it widens the box rather than raising
\hbadness or the baseline.

What to do differently: when a content PR grows a book a lot, grep its
preamble for `pnumwidth` before the strict compile. Better, add the 2.4em
setting to the remaining preambles that lack it before they cross page 999.
