---
category: Cryptography & security review
---

# Reuse an existing canonical cryptographic vector instead of retyping it

An HKDF fixed-memory acceptance test retyped RFC 5869 test case 1 even though
the same canonical expected output already existed later in the crate. One
missing byte pair made the new test fail while the implementation and existing
oracle agreed. Copy an existing reviewed in-tree vector when one is available,
then compare the new path against that same source; only introduce a separately
generated oracle when the repository has no vector for the algorithm or case.
