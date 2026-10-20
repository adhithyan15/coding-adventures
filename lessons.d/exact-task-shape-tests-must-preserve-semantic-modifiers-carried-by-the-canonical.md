---
category: Testing & coverage
---

# Exact task-shape tests must preserve semantic modifiers carried by the canonical mode

The new C2 listening contract deliberately requires overlapping speakers, but the exact expected-mode assertion was copied from C1 and omitted `overlapping`. Preserve the stronger canonical mode in the inventory and derive the exact test string from it; do not weaken content merely to satisfy a stale copied expectation.
