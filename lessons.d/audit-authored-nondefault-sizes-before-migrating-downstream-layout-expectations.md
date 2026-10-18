---
category: Testing & coverage
---

# Audit authored nondefault sizes before migrating downstream layout expectations

The first Timeline minima test proposal added size24 products but overlooked
the existing family fixture's authored root fontSize24. Its twelve complete
products still expected fixed geometry. Static review found this before any
Rust run; leaving those expectations untouched would break the later green.

Audit authored inputs and configuration substitutions across the affected
tests. Preserve their original literal bodies, prove the whole old configured
layout and Scene, then apply a closed, independently declared new expectation.
Never normalize observed production output or silently drop the old cells.
