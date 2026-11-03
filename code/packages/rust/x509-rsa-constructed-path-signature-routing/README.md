# x509-rsa-constructed-path-signature-routing

Pure public-data composition that authenticates one exact adjacent pair from a
constructed X.509 path with the repository-owned, data-driven RSA signature
router.

The caller supplies:

- one non-copyable adjacency projected from a constructed path;
- certificate-bound algorithm policy for the projected child;
- certificate-wide issuing-CA role policy for the projected issuer;
- certificate-bound RSA algorithm and key policy for that same issuer;
- a profile-index routing table; and
- the reviewed RSA public-operation limit.

Every evidence object must retain the exact decoded certificate projected by
the path. Byte-equal independent decodes are rejected, and a non-CA issuer
fails before signature verification. Success retains the path projection,
role evidence, both algorithm-policy inputs, validated issuer key, and exact
fixed-verifier result.

A follow-up can consume that routed proof with complete policy for its exact
non-terminal intermediate issuer. It rejects the construction's terminal root
and policy from any other decoded certificate object, including a byte-equal
independent decode. A second follow-up advances caller-owned path-length state
using only the retained complete policy and keeps the authenticated adjacency,
policy, and advanced state together.

An independent terminal follow-up consumes one routed proof only when its
adjacency ends at the construction's final selected issuer and reviewed-root
selection retains that same exact decoded certificate. It keeps the
construction order, certificate-wide issuing-CA policy, data-driven RSA
algorithm/key policy, fixed-verifier result, and reviewed Mozilla root
selection together. A byte-equal certificate from another decode does not
match.

This package authenticates one projected link and can retain one intermediate
path-length transition or one reviewed terminal handoff. Reviewed-root
selection is snapshot membership, not complete trust. The package does not
define full Name comparison, authenticate another link, prove prior traversal,
apply trust-anchor or name constraints, validate target identity or time,
check revocation, perform TLS or HTTP, authorize OAuth transport, or open a
network connection. It does not make concrete HTTPS or live-provider use
ready.

## Validation

```bash
bash BUILD
cargo clippy -p x509-rsa-constructed-path-signature-routing --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p x509-rsa-constructed-path-signature-routing --no-deps
```
