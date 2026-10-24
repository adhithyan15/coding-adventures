# x509-rsa-sha256-reviewed-anchor-link

This zero-external-production-dependency crate closes one evidence-mixing gap
between the repository-owned X.509 signature path and its reviewed Mozilla root
data.
It consumes:

- `VerifiedX509RsaSha256IssuerPolicyLink`, which retains the exact child and
  issuer after directed Name policy, certificate-wide critical/issuing-CA role
  policy, caller-supplied algorithm and RSA key-strength policy, and the
  child's RSA PKCS #1 v1.5 SHA-256 signature verification; and
- `SelectedReviewedWebPkiRootAnchor`, which retains the exact candidate whose
  canonical subject and SubjectPublicKeyInfo identity occurs exactly once in
  the pinned reviewed snapshot.

`bind_x509_rsa_sha256_issuer_link_to_reviewed_anchor` succeeds only when those
two non-copyable values retain the same decoded issuer certificate object.
Equal certificate bytes decoded into different objects are rejected. The
result retains both inputs so later composition cannot silently detach the
authenticated link from the reviewed anchor identity.

The two-link entry point additionally consumes one
`ValidatedX509RsaSha256AuthenticatedIntermediatePathLength` and one reviewed
parent link. It requires the first proof's complete intermediate policy and the
second proof's authenticated child to retain the same exact decoded
certificate object. Success returns a distinct non-copyable result retaining
both inputs, including the advanced path-length state. This prevents a caller
from joining independently authenticated links through byte-equal but
separately decoded intermediates.

The construction-bound entry point instead consumes one
`ValidatedX509RsaSha256ConstructedPathIntermediatePathLength`, one
`VerifiedX509RsaSha256ConstructedPathAdjacentLink`, and reviewed-root
selection. It accepts only an exact two-link construction whose retained
intermediate transition is depth zero and whose authenticated terminal link is
depth one. Both authenticated inputs must retain the identical constructed-path
object, the terminal child must be the exact retained intermediate, and the
terminal issuer must be the exact reviewed-root certificate object. Success
retains all three non-copyable proofs, including the advanced path-length
state.

## Boundary

These are fixed-shape evidence bindings, not general certificate-path
validation. The two-link result covers exactly one caller-ordered
child-to-intermediate-to-reviewed-root chain for the repository-owned RSA
SHA-256 path. The construction-bound result additionally retains the exact
constructed order, but its caller-supplied initial path-length state is not
derived from anchor policy. Neither result applies the anchor's optional name
constraints, checks target role, purpose, validity, or hostname, searches or
generalizes paths, checks revocation, performs TLS or HTTP, or authorizes OAuth
network access.
`webpki-roots` remains reviewed public data only; no external TLS, X.509, or
cryptographic implementation is linked here.

The success test uses the public ISRG Root X1 certificate solely as an
interoperability vector for the repository-owned parser and RSA verifier. Its
SHA-256 fingerprint is
`96bcec06264976f37460779acf28c5a7cfe8a3c0aae11a8ffcee05c0bddf08c6`.
