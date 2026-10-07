---
category: Testing & coverage
---

# Include the identity wrapper when asserting exact provenance serialization work

The first exact export-work test counted root, entry-map key and entry visits
but omitted compact-v1's identity wrapper. Its claimed seven-unit exact cap
failed correctly. Inspection showed four validation visits and four encoding
visits; the test now accepts eight and rejects seven.

Count version/state wrappers as well as graph and metadata payloads when
writing exact resource-boundary tests. A failing acceptance test can expose a
bad expected count rather than a production defect; inspect the visits before
changing the implementation to satisfy an incorrect expectation.
