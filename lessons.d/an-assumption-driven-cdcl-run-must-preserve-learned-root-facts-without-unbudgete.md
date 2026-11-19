---
category: Testing & coverage
---

# An assumption-driven CDCL run must preserve learned root facts without unbudgeted cleanup

An early Go implementation discarded every learned clause after a solve with
assumptions, then rebuilt root propagation with an unlimited call. A minimized
fuzz case exposed that an explicit empty clause had been reduced to a root
contradiction but was not recreated by that rebuild; `Okay` became true and a
later solve could return SAT. The rebuild also did propagation work outside the
next call's budget. The solver now keeps first-UIP resolvents: assumptions are
decision literals, not clause reasons, so resolution keeps any needed
negations in globally entailed learned clauses. Backjumps are clamped at the
active assumption prefix; root conflicts are sticky, and public propagation
records a root contradiction before returning. Keep an independent
assumptions-to-unrestricted recovery regression and assert that UNKNOWN
preserves the pending root queue without doing hidden cleanup work.
