---
category: Haskell
---

# An exact-rank recursion must initialize every accumulator

When expressing the binomial-tail rank search as a recursive accumulator loop,
the base call must initialize the best rank as well as the index, coefficient,
and accumulated tail. GHC caught a missing fourth argument before tests ran.
Name each accumulator in the helper signature and count its initial value at
the call site; run the native Cabal build immediately after changing the loop.
