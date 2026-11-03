# Changelog

## 0.4.0 - 2026-11-03

### Added

- Added an exact-object composition for one routed
  target -> intermediate -> reviewed-root construction.
- Added coverage for complete two-link evidence retention and rejection of
  proofs projected from distinct constructed-path objects.

### Security

- The composition requires exactly two issuer depths from the same retained
  construction and preserves both routed signatures, complete intermediate
  policy, advanced path-length state, and reviewed-root selection.
- The fixed shape does not validate longer paths, derive anchor constraints,
  or authorize TLS, HTTP, OAuth, or network use.

## 0.3.0 - 2026-11-03

### Added

- Added an exact-object terminal handoff from one data-driven routed
  constructed-path signature proof to reviewed Mozilla root selection.
- Added coverage for successful evidence retention, non-terminal adjacency
  rejection, and byte-equal detached reviewed-root evidence.

### Security

- The handoff requires the authenticated adjacency to end at the construction's
  final selected issuer and retains its certificate-wide issuing-CA policy.
- Reviewed snapshot membership does not grant complete path validity, apply
  trust-anchor constraints, or authorize TLS, HTTP, OAuth, or network use.

## 0.2.0 - 2026-11-03

### Added

- Added exact-object binding between one routed constructed-path signature
  proof and complete policy for its non-terminal intermediate issuer.
- Added a path-length handoff that derives its transition only from the
  retained complete intermediate policy and retains the ordered proof with the
  advanced state.
- Added coverage for terminal issuers, byte-equal detached policy, successful
  evidence retention, and exhausted non-self-issued capacity.

### Security

- Terminal roots and cross-certificate policy fail closed with payload-free
  categories.
- The composition grants no complete traversal, trust, revocation, TLS, HTTP,
  OAuth transport, or network authority.

## 0.1.0 - 2026-10-30

### Added

- Added exact-object binding between one constructed-path adjacency, child
  algorithm policy, issuer role policy, and issuer RSA algorithm/key policy.
- Added data-driven routing to the repository-owned RSA PKCS #1 v1.5 SHA-256,
  SHA-384, and SHA-512 certificate-signature verifiers.
- Added non-copyable success evidence and payload-free failure categories.

### Security

- Cross-certificate evidence and non-CA issuers fail before RSA work.
- The composition grants no complete path, trust, revocation, TLS, HTTP,
  OAuth transport, or network authority.
