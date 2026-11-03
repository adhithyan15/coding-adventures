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
policy, and advanced state together. That lower-level transition remains
available for generalized traversal code, but it cannot directly create the
fixed reviewed two-link result.

An independent terminal follow-up consumes one routed proof only when its
adjacency ends at the construction's final selected issuer and reviewed-root
selection retains that same exact decoded certificate. It keeps the
construction order, certificate-wide issuing-CA policy, data-driven RSA
algorithm/key policy, fixed-verifier result, and reviewed Mozilla root
selection together. A byte-equal certificate from another decode does not
match.

A fixed-shape traversal follow-up consumes both results for an exact
target -> intermediate -> reviewed-root construction. It requires depth zero
and depth one from the same constructed-path object, derives the RFC 5280
initial capacity of one intermediate from that exact shape, and advances it
using only the retained complete intermediate policy. It retains both routed
signature proofs and keeps the construction-derived path-length state with the
reviewed-root handoff. Callers cannot inject detached initial state. Proofs
from a separately constructed path fail even when they select the same decoded
certificates.

A target-policy follow-up consumes that routed reviewed construction together
with complete server-certificate policy. It requires the policy to retain the
construction's exact decoded target object and keeps both non-copyable inputs
together. A byte-equal independent decode does not match, so identity,
validity, purpose, end-entity role, critical-extension, and algorithm policy
cannot be spliced onto another construction.

A selected-anchor DNS-policy follow-up derives the target only from that
complete server policy and derives optional constraint bytes only from the
exact reviewed terminal retained by the same routed proof. Caller-owned limits
bound both repository-owned decoders. Unconstrained snapshot entries retain
the complete proof without DNS evidence; constrained entries require every
target dNSName to satisfy the supported permitted subtree set and retain only
a payload-free count.

An additive intermediate name-constraints follow-up consumes that complete
anchor-policy proof. It derives the exact intermediate, target, and target SAN
index only through retained evidence. An absent NameConstraints extension
succeeds without new evidence; a present extension must be critical and every
retained target dNSName and IPv4 or IPv6 iPAddress must satisfy the supported
permitted-and-excluded DNS and IP subsets together under separate caller-owned
decoder limits. Success retains the entire prior proof plus only payload-free
DNS and IP counts. The earlier DNS count remains available as compatibility
evidence only after the combined evaluation succeeds.

Reviewed-root selection is snapshot membership, not complete trust. The
fixed-shape result does not generalize to additional intermediates, derive the
initial state for a generalized traversal, define full Name comparison, apply
other GeneralName forms, excluded or non-DNS anchor constraints, check
revocation, perform hostname verification, TLS, or HTTP, authorize OAuth
transport, or open a network connection. Target identity, time, purpose, the
construction-derived fixed-shape state, and the supported reviewed-anchor DNS
and intermediate DNS/IP subsets do not make the whole path valid. This does
not make concrete HTTPS or live-provider use ready.

## Validation

```bash
bash BUILD
cargo clippy -p x509-rsa-constructed-path-signature-routing --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p x509-rsa-constructed-path-signature-routing --no-deps
```
