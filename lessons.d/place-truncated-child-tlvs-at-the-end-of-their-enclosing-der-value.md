---
category: Testing & coverage
---

# Place truncated child TLVs at the end of their enclosing DER value

A child TLV with an overstated length is not necessarily truncated when a
sibling follows it: a streaming DER decoder will consume the sibling's tag or
length bytes as the declared value. To test a bounded-child truncation, place
the malformed child last in its enclosing value or construct the enclosing
length so no following byte can satisfy it. Otherwise the fixture may be
well-framed DER with different semantics and fail at a later policy gate.
