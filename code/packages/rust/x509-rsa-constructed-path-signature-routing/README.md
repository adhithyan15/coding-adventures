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

A distinct ordered-transition entry point accepts only the stronger
Name-policy-backed reviewed-construction initializer. It requires the same
exact terminal routed proof and complete anchor-nearest intermediate policy as
the generic entry point, advances through the same shared checks, and retains
the initializer's exact validated Name-value policy, reviewed-root selection,
certificate-wide anchor role policy, construction-derived capacity, terminal
signature, child policy, and advanced state together. A generic initializer or
proof from another construction cannot produce this result. This is only the
first ordered public-certificate transition; the available Name rules remain
incomplete for RFC 5280 Name equality, and no later transition, complete path
validation, constraint, revocation, TLS, HTTP, OAuth, or network authority is
added.

A second stronger entry point consumes that policy-backed first-step result
only with the immediately target-nearer routed adjacency from the same exact
construction. Its issuer must be the already processed anchor-nearest
intermediate, and complete policy must retain its exact candidate child before
the prior path-length state advances. Success retains the exact Name-value
policy, reviewed-root and anchor-role evidence, both routed signatures, both
intermediate policies, and the twice-advanced state. Generic first-step
evidence, detached constructions, skipped depths, disconnected issuers, target
children, and cross-certificate policy fail closed. This proves exactly two
ordered intermediate steps; it does not add arbitrary traversal, complete path
or Name validation, constraints, revocation, TLS, HTTP, OAuth, or network
authority.

A stronger allocation-free accumulator now accepts only the Name-policy-backed
reviewed initializer and the same bounded ordered evidence used by the generic
accumulator. Both entry points share one private traversal implementation for
construction-derived counts, the caller ceiling, exact construction identity,
immediate depth order, issuer continuity, authenticated candidate children,
complete intermediate policy, and path-length advancement. The stronger result
retains the exact Name-value policy, reviewed-root selection, anchor-role
policy, construction-derived initial capacity, both borrowed evidence slices,
processed count, and final state. It stops before the target signature and does
not add complete RFC 5280 Name or path validation, constraints, revocation,
TLS, HTTP, OAuth, or network authority.

A matching stronger target-signature closure accepts only that Name-policy-
backed accumulator. It applies the same shared exact-construction, depth-zero,
construction-target, target-nearest issuer, and exact server-policy checks as
the generic closure, then retains the policy-backed accumulator, routed target
signature, and complete server policy together. The generic accumulator cannot
produce this result. Success preserves the exact Name-value policy,
reviewed-root selection, certificate-wide anchor-role policy, complete ordered
intermediate evidence, final path-length state, and target authentication in
one value. This still does not define complete Name comparison, validate the
whole path, apply every constraint or revocation policy, perform TLS or HTTP,
authorize OAuth transport, or grant network authority.

A matching stronger reviewed-anchor DNS-policy result accepts only the Name-
policy-backed target-signature closure. It derives the exact target, validated
SAN index, reviewed anchor, and optional constraint bytes only through that
closure, then invokes the same bounded supported permitted-DNS evaluator as the
generic result. Success retains the exact Name-value policy, reviewed-root
selection, certificate-wide anchor-role policy, complete ordered intermediate
and target-signature evidence, complete server policy, and any payload-free
constrained-name count together. A generic target closure cannot produce this
result. This covers only the supported reviewed-anchor DNS subset; it does not
define complete Name comparison, validate the whole path, apply excluded or
non-DNS anchor constraints or every intermediate constraint, check revocation,
perform TLS or HTTP, authorize OAuth transport, or grant network authority.

A matching stronger ordered-intermediate policy accepts only that Name-policy-
backed reviewed-anchor result. It derives the exact target, validated SAN
index, complete intermediate sequence, and anchor-to-target traversal order
only through the retained strong evidence, then invokes the same bounded
supported DNS/IP NameConstraints evaluator as the generic result. Success
retains the exact Name-value policy, reviewed-root selection, certificate-wide
anchor-role policy, complete ordered signature and intermediate evidence,
target authentication, reviewed-anchor DNS policy, constrained-intermediate
count, and payload-free target DNS/IP counts together. The generic ordered
policy cannot produce this stronger result. This applies supported
intermediate constraints only to exact target identities; it does not constrain
subordinate CA names, support every GeneralName, establish complete Name or
path validity, check revocation, perform TLS or HTTP, authorize OAuth
transport, or grant network authority.

A matching stronger subordinate-intermediate policy accepts only that strong
ordered target policy. It derives every complete intermediate policy, exact
certificate, anchor-to-target order, and self-issued classification only
through the retained strong evidence, then invokes the same bounded private
subordinate DNS/IP NameConstraints evaluator as the generic result. Success
retains the exact Name-value policy, reviewed-root selection, certificate-wide
anchor-role policy, complete ordered signature and intermediate evidence,
target authentication, reviewed-anchor and target-intermediate policy, and
only payload-free subordinate application and repeated-name counts together.
The generic subordinate policy cannot produce this stronger result. This does
not apply reviewed-anchor constraints to subordinate CAs, constrain subject
distinguished names, support every GeneralName, establish complete Name or
path validity, check revocation, perform TLS or HTTP, authorize OAuth
transport, or grant network authority.

A matching stronger reviewed-anchor subordinate policy accepts only that
strong subordinate-intermediate result. It derives the reviewed anchor's
optional constraint bytes, every complete intermediate certificate, traversal
order, and self-issued classification only through the retained strong
evidence, then invokes the same bounded private permitted-DNS evaluator as the
generic result. Success retains the exact Name-value policy, reviewed-root
selection, certificate-wide anchor-role policy, complete ordered signature and
intermediate evidence, all prior supported target and subordinate constraint
evaluations, and only payload-free reviewed-anchor application and repeated-DNS
counts together. The generic reviewed-anchor subordinate result cannot produce
this stronger result. This does not constrain subordinate subject distinguished
names, support every GeneralName or reviewed-anchor constraint, establish
complete Name or path validity, check revocation, perform TLS or HTTP, authorize
OAuth transport, or grant network authority.

An ordered-transition follow-up borrows the reviewed construction-derived
initializer and consumes a terminal routed proof only when both retain the same
exact constructed-path object. That terminal proof authenticates the
anchor-nearest intermediate as its exact child using the reviewed root's key;
complete intermediate policy must retain that same child. The result retains
the initializer reference, terminal signature and reviewed-root handoff,
complete child policy, and advanced state together. Proofs from another
construction and policy for another certificate fail before transition. This
API establishes the first ordered intermediate step. A second-step follow-up
consumes that result only with the immediately target-nearer routed adjacency
from the same construction. Its issuer must be the already processed
anchor-nearest intermediate, and complete policy must retain its exact
candidate child before the prior state advances. The result retains both
ordered proofs, both child policies, and the twice-advanced state. Detached
constructions, skipped depths, disconnected issuers, target children, and
cross-certificate policy fail closed. This proves exactly two intermediate
steps. An allocation-free accumulator generalizes only that ordered
path-length portion: it borrows the same initializer and reviewed terminal,
then consumes anchor-to-target slices of follow-on routed signatures and
complete intermediate policies. It derives the exact intermediate count from
the construction, requires exact evidence counts, enforces a caller-owned
nonzero intermediate ceiling, and applies every transition in immediate depth
order while retaining all borrowed evidence and the final state. It stops
before the target signature and does not establish complete path validation.
An exact target-signature closure can then consume that accumulator together
with the routed depth-zero target adjacency from the identical construction
and complete server policy for the exact decoded target. The target proof's
issuer must be the final accumulated target-nearest intermediate. Success
retains the accumulator, routed target signature, and server policy together;
detached constructions, nonzero depths, disconnected issuers, and policy from
a byte-equal independent target decode fail closed.
The reviewed-anchor DNS-policy follow-up also accepts this arbitrary-depth
closure. It derives the exact target and already-validated SAN index from the
retained server policy, and optional constraint bytes from the reviewed anchor
retained by the accumulator's terminal proof. Caller-owned decoder limits
bound the existing supported permitted-DNS evaluator. An unconstrained
snapshot entry retains the complete closure without new DNS evidence; a
constrained entry succeeds only after every retained target dNSName satisfies
that supported subset.
An ordered intermediate-policy follow-up consumes that reviewed-anchor result
and visits every complete intermediate policy retained by the accumulator in
its already-proved anchor-to-target order. The exact target and validated SAN
index still come only from the closure. Each present intermediate
NameConstraints extension must be critical, decode as the supported
permitted-and-excluded DNS/IP subset, and admit every exact target dNSName and
iPAddress under separate caller-owned decoder limits. The earlier accumulator
ceiling bounds visits. Success retains the full prior proof, the number of
constrained intermediates, and only payload-free target DNS/IP counts.
An ordered subordinate-CA follow-up consumes that target policy and derives
the same complete intermediate policies, exact certificates, traversal order,
and self-issued classifications only from retained evidence. Each
intermediate's critical supported DNS/IP constraints are applied to every
target-nearer intermediate's optional SAN, excluding non-final self-issued
subordinate CAs. An absent subordinate SAN is an empty supported-name set; a
present SAN must decode under caller-owned limits. Success retains the entire
target policy and only payload-free application and repeated-name counts.
A reviewed-anchor subordinate-CA follow-up consumes that result and derives
the reviewed snapshot's optional constraint bytes through the retained
terminal proof. An unconstrained entry succeeds without reading subordinate
SANs. A constrained entry decodes its supported permitted-DNS contents once,
then applies them to every retained non-self-issued intermediate's optional
SAN in the already-proved order. An absent SAN is an empty supported-name set;
success retains the complete prior proof and payload-free application and
repeated-DNS counts.

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
fixed-shape result, ordered transitions, intermediate accumulator, target
signature closure, arbitrary-depth reviewed-anchor policy, and ordered target
identity checks against intermediate constraints do not validate the anchor's
self-signature, constrain subordinate CA distinguished names, establish
complete generalized path validation, define full Name comparison, apply
excluded or non-DNS reviewed-anchor constraints, apply all NameConstraints or
GeneralName forms, check revocation, perform TLS or HTTP, authorize OAuth
transport, or open a network connection. Target identity, time, purpose,
routed selected-adjacency RSA signatures, construction-derived path-length
state, and the supported reviewed-anchor DNS and intermediate DNS/IP subsets
do not make the whole path valid. This does not make concrete HTTPS or
live-provider use ready.

## Validation

```bash
bash BUILD
cargo clippy -p x509-rsa-constructed-path-signature-routing --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p x509-rsa-constructed-path-signature-routing --no-deps
```
