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

## Boundary

These are fixed-shape evidence bindings, not general certificate-path
validation. The two-link result covers exactly one caller-ordered
child-to-intermediate-to-reviewed-root chain for the repository-owned RSA
SHA-256 path. It does not apply the anchor's optional name constraints, check
target role, purpose, validity, or hostname, search or generalize paths, check
revocation, perform TLS or HTTP, or authorize OAuth network access.
`webpki-roots` remains reviewed public data only; no external TLS, X.509, or
cryptographic implementation is linked here.

The success test uses the public ISRG Root X1 certificate solely as an
interoperability vector for the repository-owned parser and RSA verifier. Its
SHA-256 fingerprint is
`96bcec06264976f37460779acf28c5a7cfe8a3c0aae11a8ffcee05c0bddf08c6`.
