---
category: Testing & coverage
---

# Negative-control assertions must match the diagnostic emitted by the intended boundary

After changing a corpus guard from ordered-ID checking to exact-set checking, the replacement-ID
negative control expected the old diagnostic wording. The guard correctly rejected the ID, but the
test failed on its stale regular expression. When tightening a fail-closed boundary, run the
negative control and assert the diagnostic produced by that exact boundary; do not infer the message
from an earlier implementation.
