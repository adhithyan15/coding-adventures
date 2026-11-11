---
category: Cryptography & security review
---

# X.509 path-length test expectations must apply the intermediate constraint after the non-self-issued decrement

A longer-path test expected construction-derived capacity two to become one
after processing the first non-self-issued intermediate. The real certificate
also carried `pathLenConstraint = 0`, so RFC 5280 processing correctly tightened
the post-decrement state to zero.

When asserting a path-length transition over real certificate fixtures, inspect
both self-issued classification and BasicConstraints. The transition decrements
for a non-self-issued intermediate first, then takes the minimum with its
`pathLenConstraint`; certificate count alone is not the expected result.
