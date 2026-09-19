## HL-C409 — Malayalam A1 reading stops seventeen words before the first part

**Status: CLOSED (2026-09-19).** Re-measured after HL-C407 merged.

`npm run report:reading-reach` found Malayalam's longest connected passage at
**53 words**, while all three project-defined A1 reading parts begin at **70,
90 and 90 words**. The track therefore measured **0/3** despite already owning
enough vocabulary and grammar to sustain a longer text.

The cheapest honest repair was retrieval, not a new chapter. Chapter 69's
doorway passage now reuses water, tea, milk, dog, cat, book and the near/far
contrast from earlier owners. It grows from fifteen lines to twenty-four and
from **53 words to exactly 90**, introduces no atom, and moves Malayalam A1
reading reach to **3/3**. A focused reading-reach assertion pins both the word
count and the three reachable parts.
