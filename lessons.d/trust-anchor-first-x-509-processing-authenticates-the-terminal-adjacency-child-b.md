---
category: Cryptography & security review
---

# Trust-anchor-first X.509 processing authenticates the terminal adjacency child before applying that child's intermediate policy

An ordered path-length transition initially accepted issuer depth
`issuer_count - 2` and treated that routed link as authentication of the
anchor-nearest intermediate. The link instead verifies the certificate below
that intermediate using the intermediate's key. It leaves the intermediate
cryptographically disconnected from the reviewed trust anchor.

For the first trust-anchor-to-target step, consume the terminal adjacency at
depth `issuer_count - 1`: its issuer is the reviewed anchor and its child is
the anchor-nearest intermediate. Retain that terminal signature proof, require
complete intermediate policy for its exact child object, and only then apply
the path-length transition to the child. Describe link semantics in terms of
both child and issuer before deriving a traversal-depth formula.
