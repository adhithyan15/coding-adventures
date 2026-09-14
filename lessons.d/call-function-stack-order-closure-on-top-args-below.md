---
category: Compiler / VM / language pipeline
---

# CALL_FUNCTION stack order: closure on top, args below

Pop closure FIRST, then args via `unshift` (or equivalent). Reversing this dereferences integer arg values as heap addresses → KeyError.
