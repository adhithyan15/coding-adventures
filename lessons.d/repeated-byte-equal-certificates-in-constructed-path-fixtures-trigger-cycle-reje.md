---
category: Cryptography & security review
---

# Repeated byte-equal certificates in constructed path fixtures trigger cycle rejection even when decoded objects differ

An ordered X.509 path-length test tried to make a real two-intermediate RSA
chain by decoding the same self-signed root twice and selecting both objects at
successive issuer depths. The constructor rejected the fixture with
`CertificateCycle`: path-cycle detection compares certificate encoding, not
only Rust object identity. That is the correct construction boundary even
though later evidence composition deliberately uses exact object identity.

Do not use duplicate DER as a shortcut for a longer constructed path. Build a
non-cyclic fixture whose adjacent signatures are genuinely valid, or isolate
the ordered state transition behind test-only evidence created by an existing
validated constructor. Keep construction-cycle semantics and evidence-binding
semantics distinct when designing negative tests.
