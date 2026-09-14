---
category: Testing & coverage
---

# A mutation that does not mutate is a false exoneration

2026-09-13.

While checking whether a restored assertion caught "creating a note type through
the collection path becomes a silent no-op", the mutation tested whether the id
was in the collection **after** the upsert had already added it. The condition
was never true, nothing changed, and the run reported that no test caught the
defect.

That is worse than not testing: it is evidence pointing the wrong way, and it
would have justified deleting an assertion that review had just identified as
the only one pinning that behaviour. Written faithfully — deciding before the
reduce — the assertion caught it.

Before reading which tests a mutation fails, confirm the mutation changed the
behaviour. If a mutation reddens nothing, suspect the mutation first.
