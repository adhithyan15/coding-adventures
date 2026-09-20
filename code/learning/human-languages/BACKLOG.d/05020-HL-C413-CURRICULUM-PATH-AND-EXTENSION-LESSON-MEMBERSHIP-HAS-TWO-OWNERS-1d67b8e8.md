## HL-C413-1d67b8e8 — Curriculum path and extension lesson membership has two owners

**Status: CLOSED (2026-09-20). Tracks #15729.** The post-inventory contention
audit found Hindi's script runway repeatedly appending the same lessons to its
path and extension owners. A corpus audit found the same unambiguous reverse
membership shape across every registered curriculum.

Path shards now own exact order and explicit extension membership together.
Tagged entries distinguish extension lessons from plain path-core lessons, and
the loader derives compatible public path and extension arrays. Ambiguous
legacy shapes remain explicit, while missing, unattached, duplicate, malformed,
or dual-owned membership fails closed.
