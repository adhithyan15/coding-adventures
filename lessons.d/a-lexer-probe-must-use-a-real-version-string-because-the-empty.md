---
category: Testing & coverage
---

# A lexer probe must use a real version string, because the empty version selects a generic grammar that may lack the token under test

**What went wrong.** Security review of #17914 reported that a huge JS token
followed by many line-leading `-->` comments lexed in quadratic time. To check
the claim I probed `tokenize_javascript(&src, "")` and measured linear time,
and nearly dismissed the finding. But `""` selects javascript-lexer's
`generic` grammar, which has no `MINUS_MINUS` token. `-->` lexed as `- - >`,
the comment callback never suppressed anything, and the quadratic path never
ran. With `"es2025"` the same input allocated 8 GB more than the baseline,
exactly as the reviewer said.

**The fix.** `LexerContext::previous_token` is borrowed instead of cloned per
callback. `javascript-lexer/tests/html_close_linear.rs` counts allocated bytes
using a real version and was checked to fail on the old lexer.

**Next time.** When a probe disagrees with a reported reproduction, first make
the probe hit the reported code path: use the same entry point and a real
version, and assert that the behaviour under test actually happened (here,
that the comment was skipped). A regression test should be mutation-checked
against the old code. A wall-clock bound did not catch this bug, because a
string that fits in cache copies fast; counting allocated bytes did.
