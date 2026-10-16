---
category: Testing & coverage
---

# Count required constructor fields separately from metadata insertions in an ABI migration receipt

The pipeline ABI receipt records 62 required-field insertions and four separate
zero pipelineCount metadata additions across 21 files. A delivery helper confused
their 66 total with the field-only counter and refused before starting Cargo.
The failed preflight receipt is preserved. Read the saved schema and assert each
counter independently; retain the whole-file inverse proof when fixing a census.

TODO: what went wrong, what the fix was, and what to do differently. Replace this line.
