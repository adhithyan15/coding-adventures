---
category: Security boundaries
---

# Assessment paper evidence must have unique identities, canonical distinct paths, and regular files

The first full-mock paper gate checked whether a declared path existed, but three
counterexamples could still report false evidence: a duplicate mock ID reused an
existing omission pin, a `./` path alias made an answer key look distinct from
the learner paper, and a directory counted as a present paper. Security review
caught all three before publication.

Treat assessment evidence as an identity-and-file contract. Reject duplicate
mock IDs within each level or capstone, reject noncanonical path segments in
artifact references, and require declared artifacts to be regular files inside
their own track. Add negative tests for each bypass before trusting a green
corpus-wide audit. A pinned legacy omission is never permission to create a
second mock with the same identity.
