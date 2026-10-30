---
category: Testing & coverage
---

# Release preparation checks need pytest and full commit identities

Unittest discovery reported zero tests for TaskApp's pytest suite; it was not
validation. Run pytest explicitly and confirm the executed count (35 here).
The notes helper also rejects abbreviated commit hashes. Resolve the complete
40-character identity with git rev-parse before generating the local preview.
A passing preview remains metadata evidence, not publication evidence.
