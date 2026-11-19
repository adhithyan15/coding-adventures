---
category: Testing & coverage
---

# A pending SAT conflict must remain live until conflict analysis

When a learned asserting clause is enqueued, its propagation can discover a
second conflict above the assumption prefix. The solver stores that clause ID
as pending work for the next search iteration. Learned database reduction must
not run first: the pending clause can be unlocked as a reason, so ordinary
reason-lock protection may delete it and clear its literals. Defer reduction
and restarts until the pending conflict has been analyzed. Treat every queued
clause ID as a live reference, not only reason IDs.
