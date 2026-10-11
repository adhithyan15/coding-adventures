---
category: Compiler / VM / language pipeline
---

# Attach directive positions to preprocessor classification errors before returning them

The PREP01 engine returned `Dialect::classify` errors directly, even though it
already knew the current file and first token's line and column. A malformed
`#undef` in a rooted C file therefore surfaced as `(0, 0)`, while condition
errors retained their positions. Attach the directive's position when a
classification error has none, preserving any position the dialect supplies.
Test through the rooted C frontend so the public compile error cannot silently
regress to `(0, 0)`.
