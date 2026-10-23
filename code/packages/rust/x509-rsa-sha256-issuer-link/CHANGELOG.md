# Changelog

All notable changes to `x509-rsa-sha256-issuer-link` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-10-19

### Added

- Added one directed child-to-candidate-issuer Name-policy composition.
- Added candidate issuing-CA role and RSA public-key policy validation.
- Added exact RSA PKCS #1 v1.5 SHA-256 child-signature verification.
- Added non-copyable evidence bound to the exact child, candidate issuer, and
  validated issuer key, plus stable payload-free failure categories.
- Added a consuming composition from unique issuer-candidate selection evidence
  that authenticates the exact bound child and candidate without repeating Name
  policy.
- Added a stronger consuming composition that binds selected-candidate evidence
  to certificate-wide issuing-CA role-policy evidence for the exact same object
  and retains both proofs in a distinct non-copyable result.
- Added the strongest consuming composition, which also requires
  certificate-bound data-driven algorithm-policy evidence for that same issuer
  and retains all three proofs without treating the profile as verifier routing.
- Added an authenticated-intermediate binding that retains the strongest
  selected issuer link and complete intermediate policy only when both refer to
  the identical decoded issuer object.

### Security

- The composition keeps name equality, RSA strength, decoding work, and public
  operation limits caller-owned and explicit.
- Passing one link does not establish a path, trust anchor, validity,
  revocation, TLS, HTTP, or network authority.
- Selection-bound authentication prevents cross-certificate substitution
  between candidate selection and RSA link verification.
- Role-policy composition rejects certificate or role mismatches before RSA key
  or signature work; recognition alone remains insufficient for non-role
  critical-extension semantics.
- Full-policy composition rejects role or algorithm evidence for another
  certificate before RSA parsing or signature verification; encoded key-size
  policy remains distinct from RSA modulus-strength validation.
- Authenticated-intermediate composition rejects byte-equal policy evidence
  from a separately decoded issuer and grants no path-length, parent-link,
  complete-path, trust, revocation, TLS, HTTP, or network authority.
