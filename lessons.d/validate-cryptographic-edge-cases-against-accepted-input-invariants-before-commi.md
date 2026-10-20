---
category: Cryptography & security review
---

# Validate cryptographic edge cases against accepted-input invariants before committing a spec

The first RSA public-operation specification required a test for a permitted
public exponent with only one set bit. The upstream validated-key contract
permits only odd exponents at least three, so every accepted exponent has at
least two set bits; the requested fixture could never pass key validation. The
spec was corrected to require sparse and dense permitted odd exponents. Before
committing cryptographic test matrices, derive each proposed edge case through
the complete accepted-input invariants, especially when the implementation
consumes opaque evidence created by an earlier validation layer.
