---
category: Cryptography & security review
---

# Redaction assertions must not search for digits embedded in public algorithm names

A SHA-256 signature test tried to prove error redaction by asserting that every
rendered error omitted the substring `256`, because 256 was also a caller-owned
ceiling. The assertion necessarily failed on the public type and operation name
`Sha256`, even though no caller value was disclosed.

Redaction tests should first assert the complete stable error category, then
search only for unambiguous sensitive markers such as distinctive fixture
bytes, non-algorithmic lengths, or labeled values. A decimal value that is also
part of a public algorithm name, version, OID, or error kind is not a valid
standalone redaction oracle.
