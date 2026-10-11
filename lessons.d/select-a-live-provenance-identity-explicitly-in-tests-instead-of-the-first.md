---
category: Testing & coverage
---

# Select a live provenance identity explicitly in tests instead of the first hash-map key

The summary escaping test passed in a focused run but failed in the full suite: choosing the first HashMap key sometimes selected a tombstoned child, so adding a contribution correctly failed. Select an entry satisfying the live-identity precondition, or retain the identity returned by construction. Never use randomized map iteration order as a semantic fixture choice. Keep permanent-deletion enforcement intact.
