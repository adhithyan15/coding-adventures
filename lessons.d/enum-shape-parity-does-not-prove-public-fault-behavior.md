---
category: Testing & coverage
---

# Enum-shape parity does not prove public fault behavior

Constructing matching error-enum values in a differential test proves only
that two APIs spell a variant the same way. It does not prove that either
public API can reach the fault, that both implementations classify the same
input identically, or that a failed transition preserves complete entry
state. Execute every reachable fault through the public boundary and compare
the exact typed result plus the complete post-failure state. For a retained
compatibility variant that is genuinely unreachable, narrow the claim and
lock both the missing production constructor and the public invariant that
prevents the private failure site from being reached.
