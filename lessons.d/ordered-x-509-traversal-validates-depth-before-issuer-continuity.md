---
category: Cryptography & security review
---

# Ordered X.509 traversal validates depth before issuer continuity

The first ordered path-length accumulator checked both construction identity and
issuer-certificate continuity before checking the routed link's expected depth.
A negative test that supplied the target signature where the next intermediate
signature belonged therefore reported `EvidenceMismatch` instead of the
one-step API's stable `InvalidPathShape` result.

For ordered X.509 traversal, first prove that evidence belongs to the same
construction, then validate its required depth, and only then validate issuer
identity continuity. This keeps topology errors distinct from detached evidence
and makes composed error precedence match the reviewed one-step transition.
