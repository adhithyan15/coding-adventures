---
category: Testing & coverage
---

# An inventory-only planner fixture must preserve registered track directories when prerequisite readers validate them

The assessment-prerequisite reader correctly rejects a missing registered track
directory. A planner test copied only `core/`, `concepts/`, and `data/` to keep
its duplicated-inventory fixture small. Once the planner called the new reader,
that fixture failed on `missing track directory 'spanish'` before reaching the
deduplication assertion. The full-corpus planner tests still passed, so focused
tests that omitted this one missed the CI failure.

Keep the fixture small, but preserve each registered track as an empty directory.
The reader can then report task shapes and mocks as missing, as it would for a
real incomplete track, without copying tens of thousands of lessons. When a new
planner reader is added, run the whole planner test file (not just the new
reader tests) and audit every intentionally pruned corpus fixture against its
new filesystem inputs.
