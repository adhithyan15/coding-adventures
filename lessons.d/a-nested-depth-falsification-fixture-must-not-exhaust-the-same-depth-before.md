---
category: Cryptography & security review
---

# A nested-depth falsification fixture must not exhaust the same depth before the boundary under test

An OCSP nonce depth test lowered one shared decoder's limit, but the fixture's
mandatory CertID algorithm already reached the same depth as the embedded
nonce. Structural response decoding therefore failed before the nonce-policy
call and the test proved nothing about embedded-context preservation. When the
outer schema cannot be made shallower, decode it under its valid budget, then
pass the retained opaque container context to a separately constrained
semantic decoder. A depth-reset bug would pass that decoder; correct inherited
depth must fail at the intended boundary.
