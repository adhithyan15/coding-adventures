---
category: Compiler / VM / language pipeline
---

# In the S language `_` is the assignment operator, so it cannot appear in any identifier

(the `NAME` pattern in `s.tokens` excludes it). Builtin names borrowed from R that contain an underscore — `seq_len`, `seq_along`, `is_null` — are therefore unwriteable in S: `seq_len(4)` lexes as `seq _ len(4)` (assign `len(4)` to `seq`) and the call silently does the wrong thing rather than erroring. Use dot-style names (`is.na`, `as.character` — dots ARE valid in S names) or drop the underscore form. Hit while adding the S v2 builtin library; the failure surfaced as a runtime "expected double" panic in a test, not a parse error.
