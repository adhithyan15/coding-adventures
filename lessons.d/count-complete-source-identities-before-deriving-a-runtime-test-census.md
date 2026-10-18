---
category: Testing & coverage
---

# Count complete source identities before deriving a runtime test census

The native32 tests-only proposal initially predicted 39 distinct sources by
adding a canonical INTEGER32 companion to the 154 reviewed occurrences. That
complete program already was runtime-compile25 in the owned inventory: the
closed count is 154 occurrences and 38 distinct programs. The guarded helper
failed before writing source. Keep that failure separate from compiled tests.

Its first correction also replaced the prefix `== 39` too broadly and changed
the independent inventory's `== 394` assertion to `== 384`. That successor
likewise stopped before writes. Match the complete intended assertion, inspect
the resulting script and keep the original inventory count exact. Derive
source-set unions and intersections from full UTF8 program bytes before
claiming runtime cells; occurrence aliases remain separate unit assertions.

An attempted inline PowerShell/Python correction failed to parse because of
nested quoting. Use a saved UTF8 Python script for multi-line replacements,
preserving explicit newlines and avoiding shell interpolation.

TODO: what went wrong, what the fix was, and what to do differently. Replace this line.
