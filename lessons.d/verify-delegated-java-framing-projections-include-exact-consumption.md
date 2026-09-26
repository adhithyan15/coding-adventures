---
category: Testing & coverage
---

# Verify delegated Java framing projections include exact consumption

The first Java delegated-fixture run compared an expected Jackson `IntNode`
tag number with a normalized Java `long` `LongNode`. Their rendered JSON was
identical, but node equality correctly remained type-sensitive and 18 success
cases failed. Compare each projected tag field semantically, including the
number with `asLong()`, and separately assert that exact decoding consumed the
complete materialized input.
