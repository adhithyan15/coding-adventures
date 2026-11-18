---
category: Testing & coverage
---

# Max-sized buffer tail tests must use the selected algorithm's output length

A `CertificateVerify` framing test allocated the SHA-384 maximum plus a fixed
sentinel tail, then asserted that every case left exactly that fixed tail. The
SHA-256 case correctly wrote fewer bytes, so it preserved both the fixed tail
and the unused SHA-384 capacity and the assertion failed.

When one maximum buffer serves multiple fixed hash profiles, derive the written
prefix from the returned evidence and assert that every remaining byte retains
the sentinel. Do not assume every admitted algorithm consumes the maximum
profile's length.
