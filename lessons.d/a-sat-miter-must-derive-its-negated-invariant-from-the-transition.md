---
category: Testing & coverage
---

# A SAT miter must derive its negated invariant from the transition

An arena miter once fixed the link target to the append index, excluded every
out-of-range index from the field domain, and then asked whether the same link
could be one-past-count. UNSAT only repeated the assumptions; it did not prove
that the append transition kept the link in range. Model the old count, append
index, post-count, source index, and link target as shared variables. Encode the
transition equations and pre-state assumptions, then assert the negation of
the derived property. A negative control should relax an actual precondition or
mutate the transition and produce SAT.
