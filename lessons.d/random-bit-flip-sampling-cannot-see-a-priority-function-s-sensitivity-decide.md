---
category: Testing & coverage
---

# Random bit-flip sampling cannot see a priority function's sensitivity: decide a small cone exactly with a SAT query before calling an instance dead

The VOS30 guard G2 measures liveness on behaviour: flip each declared input bit on 256 random assignments and
count the bits whose flip ever changes an output. For `pick` (the highest set member of an N-bit mask) a
bit matters only when no higher bit is set, which random assignments reach for about nine bits of any width,
so a 64-bit `pick` measured 7 sensitive bits of 64 and was called dead. The sampling is the cheap stage; the
spec's own remedy ("where an instance is small, a SAT query replaces the sampling") is the exact one.
`inst::sensitive_bits` now decides every undecided bit of a cone up to 4,000 ANDs with the query "exists an
assignment where flipping this bit changes an output", on the cone and a twin of it. A larger cone is still
sampled and the row says so.

Do the same wherever a "no input was found that does X" check is built from random sampling: a function that
depends on a rare condition looks dead to sampling and alive to a solver.
