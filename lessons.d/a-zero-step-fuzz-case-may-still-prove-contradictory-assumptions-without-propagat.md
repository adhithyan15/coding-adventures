---
category: Testing & coverage
---

# A zero-step fuzz case may still prove contradictory assumptions without propagation work

A stateful fuzz harness initially required every zero-propagation-budget
operation to return UNKNOWN. A minimized byte sequence generated opposite
assumptions, so the solver could prove UNSAT directly from the assumption
prefix without consuming a propagation step. That expectation was stronger
than the budget contract and made a correct result look like a solver failure.
For bounded solver operations, compare every non-UNKNOWN result with the
independent formula-plus-assumptions oracle; require UNKNOWN only when the test
constructs work that actually exceeds the specified budget. Keep separate
zero-budget cases that prove pending propagation returns UNKNOWN and that a
later unbounded call recovers.
