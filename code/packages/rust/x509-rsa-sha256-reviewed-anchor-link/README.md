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

## Boundary

This is one-link evidence binding, not general certificate-path validation. It
does not apply the anchor's optional name constraints, check target role,
purpose, validity, or hostname, apply path-length or other constraints,
construct a multi-hop path, check revocation, perform TLS or HTTP, or authorize
OAuth network access. `webpki-roots` remains reviewed public data only; no
external TLS, X.509, or cryptographic implementation is linked here.

The success test uses the public ISRG Root X1 certificate solely as an
interoperability vector for the repository-owned parser and RSA verifier. Its
SHA-256 fingerprint is
`96bcec06264976f37460779acf28c5a7cfe8a3c0aae11a8ffcee05c0bddf08c6`.
